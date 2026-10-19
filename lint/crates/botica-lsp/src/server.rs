//! The loop of the server: the messages of the editor, what the extension rules find and what the
//! rules of project scope find, each published with the findings of its document.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::Scope;
use std::time::Instant;

use botica_core::{Buffer, Diagnostic};
use serde_json::{json, Value};

use crate::extensions::{work, Event, Job};
use crate::linter::{Linter, When};
use crate::wire::{offset_of, path_of, publish, reply, uri_of, write_message, Columns, Encoding};
use crate::{Extensions, Options};

/// What the server knows of an open document: its buffer, what the engine found as it was typed
/// and when it was last saved, and what each extension rule found. `revision` counts the texts it
/// has had, and `due` is when the extensions are given this one, if they have not been.
struct Document {
    buffer: Buffer,
    revision: u64,
    typed: Vec<Diagnostic>,
    saved: Vec<Diagnostic>,
    found: BTreeMap<String, Vec<Diagnostic>>,
    due: Option<Instant>,
}

impl Document {
    fn new(buffer: Buffer, revision: u64) -> Self {
        Self {
            buffer,
            revision,
            typed: Vec::new(),
            saved: Vec::new(),
            found: BTreeMap::new(),
            due: None,
        }
    }

    fn findings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.typed
            .iter()
            .chain(&self.saved)
            .chain(self.found.values().flatten())
    }

    /// Lints the buffer again: the findings shown as the text changes, and with `saving` those
    /// shown at save too.
    fn lint(&mut self, linter: &dyn Linter, saving: bool) {
        let (mut typed, mut saved) = (Vec::new(), Vec::new());
        for found in linter.lint(&self.buffer) {
            match linter.when(&found.rule) {
                Some(When::Type) => typed.push(found),
                Some(When::Save) => saved.push(found),
                None => {}
            }
        }
        self.typed = typed;
        if saving {
            self.saved = saved;
        }
    }
}

pub struct Server<'scope, 'env, W: Write> {
    output: W,
    root: &'env Path,
    linter: Box<dyn Linter + 'env>,
    options: Options<'env>,
    scope: &'scope Scope<'scope, 'env>,
    events: Sender<Event>,
    /// Whether the editor asked for the extensions in `initialize`.
    asked: bool,
    encoding: Encoding,
    documents: HashMap<String, Document>,
    workers: HashMap<String, Sender<Job>>,
    /// What the rules of project scope found, by URI.
    project: HashMap<String, Vec<Diagnostic>>,
    /// The number of the last run of the rules of project scope: an older run is not shown.
    generation: u64,
}

impl<'scope, 'env, W: Write> Server<'scope, 'env, W> {
    pub fn run(&mut self, inbox: &Receiver<Event>) -> io::Result<()> {
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
                Event::Warn(message) => self.warn(&message)?,
                Event::Project { generation, found } => {
                    if generation == self.generation {
                        self.show_project(found)?;
                    }
                }
            }
        }
    }

    fn warn(&mut self, message: &str) -> io::Result<()> {
        write_message(
            &mut self.output,
            &json!({"jsonrpc": "2.0", "method": "window/showMessage",
                    "params": {"type": 2, "message": message}}),
        )
    }

    /// Answers a message; `true` when it is `exit`.
    fn handle(&mut self, message: &Value) -> io::Result<bool> {
        let method = message["method"].as_str().unwrap_or_default();
        let params = &message["params"];
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        match method {
            "exit" => return Ok(true),
            "initialize" => self.initialize(&message["id"], params)?,
            "shutdown" => write_message(&mut self.output, &reply(&message["id"], Value::Null))?,
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or_default();
                self.open(uri, text)?;
            }
            "textDocument/didChange" => self.change(uri, &params["contentChanges"])?,
            "textDocument/didSave" => self.save(uri)?,
            "textDocument/didClose" => {
                self.documents.remove(uri);
                self.publish(uri)?;
            }
            "workspace/didChangeWatchedFiles" => {
                let configures = params["changes"].as_array().is_some_and(|changes| {
                    changes.iter().any(|change| {
                        let uri = change["uri"].as_str().unwrap_or_default();
                        self.linter.configures(&path_of(uri, self.root))
                    })
                });
                if configures {
                    self.reload()?;
                }
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

    fn initialize(&mut self, id: &Value, params: &Value) -> io::Result<()> {
        self.asked = params["initializationOptions"]["extensions"] == json!(true);
        self.encoding = Encoding::chosen(&params["capabilities"]["general"]["positionEncodings"]);
        write_message(
            &mut self.output,
            &reply(
                id,
                json!({
                    "capabilities": {
                        "positionEncoding": self.encoding.name(),
                        "textDocumentSync": {
                            "openClose": true,
                            "change": 2,
                            "save": {"includeText": false},
                        },
                    },
                    "serverInfo": {"name": "botica"},
                }),
            ),
        )
    }

    /// The extensions that the server runs: those it has and the editor asked for.
    fn extensions(&self) -> Option<&'env dyn Extensions> {
        self.options
            .extensions
            .filter(|_| self.options.enabled || self.asked)
    }

    fn next_revision(&self, uri: &str) -> u64 {
        self.documents
            .get(uri)
            .map_or(1, |known| known.revision + 1)
    }

    /// Lints and publishes a document the editor opened, and gives it to the extensions at once.
    fn open(&mut self, uri: &str, text: &str) -> io::Result<()> {
        let path = path_of(uri, self.root);
        let mut document = Document::new(self.linter.open(&path, text), self.next_revision(uri));
        document.lint(&*self.linter, true);
        self.documents.insert(uri.to_owned(), document);
        self.publish(uri)?;
        self.hand_over(uri, &[When::Type, When::Save]);
        Ok(())
    }

    /// Applies the changes in their order, a range to replace or a whole text, and lints the rules
    /// shown as the text changes. The extensions get the text after the debounce: the next change
    /// may come before.
    fn change(&mut self, uri: &str, changes: &Value) -> io::Result<()> {
        let Some(changes) = changes.as_array() else {
            return Ok(());
        };
        let extensions = self.extensions();
        let encoding = self.encoding;
        let due = Instant::now() + self.options.debounce;
        let (linter, root) = (&*self.linter, self.root);
        let document = self
            .documents
            .entry(uri.to_owned())
            .or_insert_with(|| Document::new(linter.open(&path_of(uri, root), ""), 0));
        for change in changes {
            let text = change["text"].as_str().unwrap_or_default();
            match change.get("range").filter(|range| !range.is_null()) {
                Some(range) => {
                    let held = document.buffer.text();
                    let start = offset_of(held, &range["start"], encoding);
                    let end = offset_of(held, &range["end"], encoding);
                    document.buffer.edit(start, end, text);
                }
                None => document.buffer.replace(text),
            }
        }
        document.revision += 1;
        document.lint(&*self.linter, false);
        if let Some(extensions) = extensions {
            for key in extensions.at(document.buffer.path(), When::Type) {
                document.found.remove(&key);
            }
            document.due = Some(due);
        }
        self.publish(uri)
    }

    /// At a save: the configuration is read again when the file holds it; otherwise the document
    /// is linted with the rules shown at save, and the rules of project scope run.
    fn save(&mut self, uri: &str) -> io::Result<()> {
        if self.linter.configures(&path_of(uri, self.root)) {
            return self.reload();
        }
        if let Some(document) = self.documents.get_mut(uri) {
            document.lint(&*self.linter, true);
            self.publish(uri)?;
            self.hand_over(uri, &[When::Save]);
        }
        self.run_project()
    }

    /// Builds the linter again and lints every open document with it. When it cannot be built the
    /// editor is told why, and the rules stay as they were.
    fn reload(&mut self) -> io::Result<()> {
        let Some(reload) = self.options.reload else {
            return Ok(());
        };
        match reload() {
            Ok(linter) => self.linter = linter,
            Err(reason) => {
                return self.warn(&format!(
                    "botica: the configuration was not read again ({reason}); the rules stay as \
                     they were"
                ));
            }
        }
        let uris: Vec<String> = self.documents.keys().cloned().collect();
        for uri in uris {
            if let Some(document) = self.documents.get_mut(&uri) {
                document.buffer = self
                    .linter
                    .open(document.buffer.path(), document.buffer.text());
                document.lint(&*self.linter, true);
            }
            self.publish(&uri)?;
        }
        self.run_project()
    }

    /// Starts the rules of project scope on a thread of their own; with none active, what they
    /// found before is cleared.
    fn run_project(&mut self) -> io::Result<()> {
        self.generation += 1;
        let Some(run) = self.linter.project() else {
            return self.show_project(Vec::new());
        };
        let (generation, events) = (self.generation, self.events.clone());
        std::thread::spawn(move || {
            let found = run();
            let _ = events.send(Event::Project { generation, found });
        });
        Ok(())
    }

    /// Shows what the rules of project scope found, and clears what they found before.
    fn show_project(&mut self, found: Vec<Diagnostic>) -> io::Result<()> {
        let mut by_uri: HashMap<String, Vec<Diagnostic>> = HashMap::new();
        for finding in found {
            let uri = self
                .documents
                .iter()
                .find(|(_, document)| document.buffer.path() == finding.path)
                .map_or_else(|| uri_of(&finding.path, self.root), |(uri, _)| uri.clone());
            by_uri.entry(uri).or_default().push(finding);
        }
        let uris: BTreeSet<String> = self.project.keys().chain(by_uri.keys()).cloned().collect();
        self.project = by_uri;
        for uri in uris {
            self.publish(&uri)?;
        }
        Ok(())
    }

    /// Publishes all that is found in a document, open or not.
    fn publish(&mut self, uri: &str) -> io::Result<()> {
        let project = self.project.get(uri).map_or(&[][..], Vec::as_slice);
        let (findings, read): (Vec<Diagnostic>, Option<String>) = match self.documents.get(uri) {
            Some(document) => (document.findings().chain(project).cloned().collect(), None),
            None if project.is_empty() => (Vec::new(), None),
            None => (
                project.to_vec(),
                std::fs::read_to_string(self.root.join(path_of(uri, self.root))).ok(),
            ),
        };
        let text = self
            .documents
            .get(uri)
            .map(|document| document.buffer.text())
            .or(read.as_deref());
        publish(
            &mut self.output,
            uri,
            &findings,
            Columns {
                text,
                encoding: self.encoding,
            },
        )
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
            self.hand_over(&uri, &[When::Type]);
        }
    }

    /// Gives the document to each extension rule that reads it and shows its findings at one of
    /// `whens`, starting the thread of the rule the first time.
    fn hand_over(&mut self, uri: &str, whens: &[When]) {
        let Some(extensions) = self.extensions() else {
            return;
        };
        let Some(document) = self.documents.get(uri) else {
            return;
        };
        let (path, text, revision) = (
            document.buffer.path().to_owned(),
            document.buffer.text().to_owned(),
            document.revision,
        );
        for &when in whens {
            for key in extensions.at(&path, when) {
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
        self.publish(uri)
    }
}

/// A server that has read nothing yet.
pub fn fresh<'scope, 'env, W: Write>(
    output: W,
    root: &'env Path,
    linter: &'env dyn Linter,
    options: Options<'env>,
    scope: &'scope Scope<'scope, 'env>,
    events: Sender<Event>,
) -> Server<'scope, 'env, W> {
    Server {
        output,
        root,
        linter: Box::new(linter),
        options,
        scope,
        events,
        asked: false,
        encoding: Encoding::Utf16,
        documents: HashMap::new(),
        workers: HashMap::new(),
        project: HashMap::new(),
        generation: 0,
    }
}
