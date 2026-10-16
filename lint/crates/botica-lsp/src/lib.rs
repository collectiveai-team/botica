//! The Language Server Protocol front-end of botica: the findings of the rules for the buffers
//! of an editor, framed as the protocol frames them (`Content-Length` headers, JSON-RPC 2.0).
//!
//! Each `didOpen` and `didChange` lints the text the editor sends with the engine and publishes
//! the findings, and `didClose` clears them. The extension rules, when they are asked for, run in
//! their own processes, one thread for each rule: the text of a document is sent to them when it
//! has stayed as it is for the debounce, and what they find is published after the findings of
//! the engine, together with them. Columns are counted in characters, which are the units of the
//! protocol for text in the Basic Multilingual Plane.

mod extensions;
mod wire;

use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::Scope;
use std::time::{Duration, Instant};

use botica_core::Diagnostic;
use serde_json::{json, Value};

use extensions::{work, Event, Job};
pub use extensions::{Extensions, Process, MAX_RESTARTS};
use wire::{path_of, publish, read_message, reply, write_message};

/// What lints a document: its path from the root of the project, and its text.
pub type Lint<'a> = &'a dyn Fn(&str, &str) -> Vec<Diagnostic>;

/// How long the text of a document has to stay as it is before the extensions lint it.
pub const DEBOUNCE: Duration = Duration::from_millis(250);

/// What the server runs besides the engine.
pub struct Options<'a> {
    /// The extension rules of the project, if there are any to run.
    pub extensions: Option<&'a dyn Extensions>,
    /// Whether the extensions run although the editor did not ask for them.
    pub enabled: bool,
    pub debounce: Duration,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            extensions: None,
            enabled: false,
            debounce: DEBOUNCE,
        }
    }
}

/// What the server knows of an open document: its text, what the engine found and what each rule
/// found. `revision` counts the texts it has had, and `due` is when the extensions are given this
/// one, if they have not been.
struct Document {
    path: String,
    text: String,
    revision: u64,
    engine: Vec<Diagnostic>,
    found: BTreeMap<String, Vec<Diagnostic>>,
    due: Option<Instant>,
}

impl Document {
    fn findings(&self) -> Vec<Diagnostic> {
        self.engine
            .iter()
            .chain(self.found.values().flatten())
            .cloned()
            .collect()
    }
}

struct Server<'scope, 'env, W: Write> {
    output: W,
    root: &'env Path,
    lint: Lint<'env>,
    options: Options<'env>,
    scope: &'scope Scope<'scope, 'env>,
    events: Sender<Event>,
    /// Whether the editor asked for the extensions in `initialize`.
    asked: bool,
    documents: HashMap<String, Document>,
    workers: HashMap<String, Sender<Job>>,
}

impl<'scope, 'env, W: Write> Server<'scope, 'env, W> {
    fn run(&mut self, inbox: &Receiver<Event>) -> io::Result<()> {
        loop {
            let event = match self.next_due() {
                Some(due) => {
                    match inbox.recv_timeout(due.saturating_duration_since(Instant::now())) {
                        Ok(event) => event,
                        Err(RecvTimeoutError::Timeout) => {
                            self.hand_over_due();
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => return Ok(()),
                    }
                }
                None => match inbox.recv() {
                    Ok(event) => event,
                    Err(_) => return Ok(()),
                },
            };
            match event {
                Event::Input(Ok(Some(message))) => {
                    if self.handle(&message)? {
                        return Ok(());
                    }
                }
                Event::Input(Ok(None)) => return Ok(()),
                Event::Input(Err(error)) => return Err(error),
                Event::Found {
                    key,
                    uri,
                    revision,
                    found,
                } => self.add(key, &uri, revision, found)?,
                Event::Warn(message) => write_message(
                    &mut self.output,
                    &json!({"jsonrpc": "2.0", "method": "window/showMessage",
                            "params": {"type": 2, "message": message}}),
                )?,
            }
        }
    }

    /// Answers a message; `true` when it is `exit`.
    fn handle(&mut self, message: &Value) -> io::Result<bool> {
        let method = message["method"].as_str().unwrap_or_default();
        let params = &message["params"];
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        match method {
            "exit" => return Ok(true),
            "initialize" => {
                self.asked = params["initializationOptions"]["extensions"] == json!(true);
                write_message(
                    &mut self.output,
                    &reply(
                        &message["id"],
                        json!({
                            "capabilities": {"textDocumentSync": {"openClose": true, "change": 1}},
                            "serverInfo": {"name": "botica"},
                        }),
                    ),
                )?;
            }
            "shutdown" => write_message(&mut self.output, &reply(&message["id"], Value::Null))?,
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or_default();
                self.lint_and_publish(uri, text, false)?;
            }
            "textDocument/didChange" => {
                let changes = params["contentChanges"].as_array();
                if let Some(text) = changes
                    .and_then(|changes| changes.last())
                    .and_then(|change| change["text"].as_str())
                {
                    self.lint_and_publish(uri, text, true)?;
                }
            }
            "textDocument/didClose" => {
                self.documents.remove(uri);
                publish(&mut self.output, uri, &[])?;
            }
            _ if message.get("id").is_some() => write_message(
                &mut self.output,
                &json!({"jsonrpc": "2.0", "id": message["id"],
                        "error": {"code": -32601, "message": method}}),
            )?,
            _ => {}
        }
        Ok(false)
    }

    /// The extensions that the server runs: those it has and the editor asked for.
    fn extensions(&self) -> Option<&'env dyn Extensions> {
        self.options
            .extensions
            .filter(|_| self.options.enabled || self.asked)
    }

    /// Publishes what the engine finds in `text`. The extensions get the text at once, or after
    /// the debounce when it is a change: the next change may come before.
    fn lint_and_publish(&mut self, uri: &str, text: &str, change: bool) -> io::Result<()> {
        let path = path_of(uri, self.root);
        let engine = (self.lint)(&path, text);
        publish(&mut self.output, uri, &engine)?;
        let revision = self
            .documents
            .get(uri)
            .map_or(1, |known| known.revision + 1);
        let debounced = change && self.extensions().is_some();
        self.documents.insert(
            uri.to_owned(),
            Document {
                path,
                text: text.to_owned(),
                revision,
                engine,
                found: BTreeMap::new(),
                due: debounced.then(|| Instant::now() + self.options.debounce),
            },
        );
        if !change {
            self.hand_over(uri);
        }
        Ok(())
    }

    fn next_due(&self) -> Option<Instant> {
        self.documents
            .values()
            .filter_map(|document| document.due)
            .min()
    }

    fn hand_over_due(&mut self) {
        let now = Instant::now();
        let due: Vec<String> = self
            .documents
            .iter()
            .filter(|(_, document)| document.due.is_some_and(|due| due <= now))
            .map(|(uri, _)| uri.clone())
            .collect();
        for uri in due {
            if let Some(document) = self.documents.get_mut(&uri) {
                document.due = None;
            }
            self.hand_over(&uri);
        }
    }

    /// Gives the document to each extension rule that reads it, starting the thread of the rule
    /// the first time.
    fn hand_over(&mut self, uri: &str) {
        let Some(extensions) = self.extensions() else {
            return;
        };
        let Some(document) = self.documents.get(uri) else {
            return;
        };
        let (path, text, revision) = (
            document.path.clone(),
            document.text.clone(),
            document.revision,
        );
        for key in extensions.at(&path) {
            let jobs = self.workers.entry(key.clone()).or_insert_with(|| {
                let (jobs, queue) = mpsc::channel();
                let events = self.events.clone();
                self.scope
                    .spawn(move || work(&key, extensions, &queue, &events));
                jobs
            });
            let _ = jobs.send(Job {
                uri: uri.to_owned(),
                revision,
                path: path.clone(),
                text: text.clone(),
            });
        }
    }

    /// Adds what a rule found to the findings of the document, unless the document is closed or
    /// has another text now.
    fn add(
        &mut self,
        key: String,
        uri: &str,
        revision: u64,
        found: Vec<Diagnostic>,
    ) -> io::Result<()> {
        let Some(document) = self.documents.get_mut(uri) else {
            return Ok(());
        };
        if document.revision != revision {
            return Ok(());
        }
        document.found.insert(key, found);
        publish(&mut self.output, uri, &document.findings())
    }
}

/// Serves the messages of `input` until `exit` or the end of `input`. `lint` lints a document by
/// its path from `root`.
///
/// # Errors
/// Fails when `input` or `output` does, or when a message is not framed or is not JSON.
pub fn serve_on(
    input: impl BufRead + Send + 'static,
    output: impl Write,
    root: &Path,
    lint: Lint<'_>,
) -> io::Result<()> {
    serve_with(input, output, root, lint, Options::default())
}

/// As [`serve_on`], and the extension rules of `options` lint the documents too.
///
/// # Errors
/// As [`serve_on`].
pub fn serve_with(
    mut input: impl BufRead + Send + 'static,
    output: impl Write,
    root: &Path,
    lint: Lint<'_>,
    options: Options<'_>,
) -> io::Result<()> {
    let (events, inbox) = mpsc::channel();
    let reader = events.clone();
    std::thread::spawn(move || loop {
        let message = read_message(&mut input);
        let last = !matches!(message, Ok(Some(_)));
        if reader.send(Event::Input(message)).is_err() || last {
            break;
        }
    });
    std::thread::scope(|scope| {
        let mut server = Server {
            output,
            root,
            lint,
            options,
            scope,
            events,
            asked: false,
            documents: HashMap::new(),
            workers: HashMap::new(),
        };
        let served = server.run(&inbox);
        drop(server);
        served
    })
}

/// Serves on stdin and stdout.
///
/// # Errors
/// As [`serve_on`].
pub fn serve(root: &Path, lint: Lint<'_>) -> io::Result<()> {
    serve_on(
        io::BufReader::new(io::stdin()),
        io::stdout().lock(),
        root,
        lint,
    )
}
