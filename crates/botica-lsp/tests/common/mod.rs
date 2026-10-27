//! What the tests of the extension rules in the server share: an editor that talks to the
//! server through a channel, and extension rules that are fakes, which note what they are asked
//! and answer as the test says.

#![allow(dead_code)]

use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use botica_core::{Diagnostic, Severity};
use botica_lsp::{Extensions, Linter, Options, Plain, Process, When};
use serde_json::{json, Value};

pub const DEADLINE: Duration = Duration::from_secs(5);

/// What the editor writes reaches the server through a channel, so a test sends a message when it
/// wants to and the server sees the end of its input when the test drops the sender.
pub struct Pipe {
    chunks: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.pending.is_empty() {
            match self.chunks.recv() {
                Ok(chunk) => self.pending = chunk,
                Err(_) => return Ok(0),
            }
        }
        let length = out.len().min(self.pending.len());
        out[..length].copy_from_slice(&self.pending[..length]);
        self.pending.drain(..length);
        Ok(length)
    }
}

#[derive(Clone, Default)]
pub struct Written(Arc<Mutex<Vec<u8>>>);

impl Write for Written {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("the output").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn unframe(bytes: &[u8]) -> Vec<Value> {
    let (messages, rest) = whole_messages(bytes);
    assert!(rest.is_empty(), "a message is cut: {rest:?}");
    messages
}

/// The messages whose frame is whole, and the bytes after them. The server writes a message in
/// several pieces, so an editor that reads while it writes may see the start of one.
fn whole_messages(mut bytes: &[u8]) -> (Vec<Value>, &[u8]) {
    let mut messages = Vec::new();
    loop {
        let Some(split) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            return (messages, bytes);
        };
        let length: usize = std::str::from_utf8(&bytes[..split])
            .expect("UTF-8")
            .strip_prefix("Content-Length: ")
            .expect("a Content-Length")
            .parse()
            .expect("a number");
        let Some(body) = bytes.get(split + 4..split + 4 + length) else {
            return (messages, bytes);
        };
        messages.push(serde_json::from_slice(body).expect("JSON"));
        bytes = &bytes[split + 4 + length..];
    }
}

pub struct Client {
    input: Option<Sender<Vec<u8>>>,
    output: Written,
}

impl Client {
    pub fn send(&self, message: &Value) {
        let body = message.to_string();
        let framed = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let _ = self
            .input
            .as_ref()
            .expect("the input is open")
            .send(framed.into_bytes());
    }

    pub fn all(&self) -> Vec<Value> {
        whole_messages(&self.output.0.lock().expect("the output")).0
    }

    /// Waits until the server has written `count` messages in all.
    pub fn wait_for(&self, count: usize) -> Vec<Value> {
        let limit = Instant::now() + DEADLINE;
        loop {
            let written = self.all();
            if written.len() >= count {
                return written;
            }
            assert!(
                Instant::now() < limit,
                "the server wrote {} messages, not {count}",
                written.len()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    pub fn end(&mut self) {
        self.input = None;
    }
}

pub fn finding(rule: &str, message: &str) -> Diagnostic {
    Diagnostic {
        path: "a.py".to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start: 0,
        end: 1,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 2,
        file_level: false,
        measure: None,
        message: message.to_owned(),
    }
}

/// What one fake process was asked: the key of its rule, the path and the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    pub key: String,
    pub path: String,
    pub text: String,
}

pub type Answer = Box<dyn Fn(&Asked, usize) -> Result<Vec<Diagnostic>, String> + Send + Sync>;

/// Extension rules that read `.py` files as `a` and `.ts` files as `b`, unless `keys` says
/// otherwise. `answer` is given each request and how many that rule has had before it.
pub struct Fake {
    pub keys: fn(&str) -> Vec<String>,
    /// The rules whose findings show at save; none unless a test says so.
    pub saved_keys: fn(&str) -> Vec<String>,
    pub answer: Answer,
    /// Why a process does not start, when none does.
    pub start_error: Option<&'static str>,
    pub starts: Mutex<Vec<String>>,
    pub finished: Mutex<Vec<String>>,
    pub asked: Mutex<Vec<Asked>>,
}

pub fn by_extension(path: &str) -> Vec<String> {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("py") => vec!["a".to_owned()],
        Some("ts") => vec!["b".to_owned()],
        _ => Vec::new(),
    }
}

impl Fake {
    pub fn new(
        answer: impl Fn(&Asked, usize) -> Result<Vec<Diagnostic>, String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            keys: by_extension,
            saved_keys: |_| Vec::new(),
            answer: Box::new(answer),
            start_error: None,
            starts: Mutex::default(),
            finished: Mutex::default(),
            asked: Mutex::default(),
        }
    }

    pub fn starts(&self) -> Vec<String> {
        let mut starts = self.starts.lock().expect("starts").clone();
        starts.sort();
        starts
    }

    pub fn finished(&self) -> Vec<String> {
        let mut finished = self.finished.lock().expect("finished").clone();
        finished.sort();
        finished
    }

    pub fn asked(&self) -> Vec<Asked> {
        self.asked.lock().expect("asked").clone()
    }

    /// Waits until the rules have been asked `count` times in all.
    pub fn wait_asked(&self, count: usize) {
        let limit = Instant::now() + DEADLINE;
        while self.asked.lock().expect("asked").len() < count {
            assert!(
                Instant::now() < limit,
                "the rules were not asked {count} times"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

pub struct FakeProcess<'a> {
    fake: &'a Fake,
    key: String,
}

impl Process for FakeProcess<'_> {
    fn lint(&mut self, path: &str, text: &str) -> Result<Vec<Diagnostic>, String> {
        let asked = Asked {
            key: self.key.clone(),
            path: path.to_owned(),
            text: text.to_owned(),
        };
        let nth = {
            let mut all = self.fake.asked.lock().expect("asked");
            all.push(asked.clone());
            all.iter().filter(|each| each.key == self.key).count()
        };
        (self.fake.answer)(&asked, nth)
    }

    fn finish(self: Box<Self>) {
        self.fake.finished.lock().expect("finished").push(self.key);
    }
}

impl Extensions for Fake {
    fn at(&self, path: &str, when: When) -> Vec<String> {
        match when {
            When::Type => (self.keys)(path),
            When::Save => (self.saved_keys)(path),
        }
    }

    fn start(&self, key: &str) -> Result<Box<dyn Process + '_>, String> {
        self.starts.lock().expect("starts").push(key.to_owned());
        if let Some(reason) = self.start_error {
            return Err(reason.to_owned());
        }
        Ok(Box::new(FakeProcess {
            fake: self,
            key: key.to_owned(),
        }))
    }
}

/// A point that threads wait at until a test opens it.
#[derive(Default)]
pub struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    pub fn wait(&self) {
        let limit = Instant::now() + DEADLINE;
        let mut open = self.open.lock().expect("the gate");
        while !*open {
            let (guard, timeout) = self
                .changed
                .wait_timeout(open, limit.saturating_duration_since(Instant::now()))
                .expect("the gate");
            open = guard;
            assert!(!timeout.timed_out(), "the gate was never opened");
        }
    }

    pub fn open(&self) {
        *self.open.lock().expect("the gate") = true;
        self.changed.notify_all();
    }
}

/// Serves `test` as the editor and returns every message the server wrote. The server runs on
/// this thread, the editor on another one; the server ends with the input of the editor.
pub fn serve(
    extensions: &Fake,
    options: Options<'_>,
    test: impl FnOnce(&mut Client) + Send,
) -> Vec<Value> {
    let options = Options {
        extensions: Some(extensions),
        ..options
    };
    serve_linter(
        &Plain(&|_, _| vec![finding("engine-rule", "from the engine")]),
        options,
        test,
    )
}

/// As [`serve`], with `linter` for the documents.
pub fn serve_linter(
    linter: &dyn Linter,
    options: Options<'_>,
    test: impl FnOnce(&mut Client) + Send,
) -> Vec<Value> {
    let (input, chunks) = channel();
    let output = Written::default();
    let mut client = Client {
        input: Some(input),
        output: output.clone(),
    };
    std::thread::scope(|scope| {
        let editor = scope.spawn(move || test(&mut client));
        let served = botica_lsp::serve_linter(
            BufReader::new(Pipe {
                chunks,
                pending: Vec::new(),
            }),
            output.clone(),
            Path::new("/work/app"),
            linter,
            options,
        );
        editor.join().expect("the editor ends");
        served.expect("the server serves");
    });
    let bytes = output.0.lock().expect("the output").clone();
    unframe(&bytes)
}

pub fn initialize(options: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
           "params": {"capabilities": {}, "initializationOptions": options}})
}

pub fn open(uri: &str, version: u32, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "python", "version": version, "text": text}}})
}

pub fn change(uri: &str, version: u32, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": version},
        "contentChanges": [{"text": text}]}})
}

pub fn exit() -> Value {
    json!({"jsonrpc": "2.0", "method": "exit"})
}

pub const A_PY: &str = "file:///work/app/a.py";

pub fn messages_of(published: &Value) -> Vec<String> {
    published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|found| found["message"].as_str().expect("a message").to_owned())
        .collect()
}

pub fn on(extensions: &Fake) -> Options<'_> {
    Options {
        extensions: Some(extensions),
        enabled: false,
        debounce: Duration::from_millis(60),
        ..Options::default()
    }
}

pub fn asking() -> Value {
    json!({"extensions": true})
}

pub fn shutdown(id: u32) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "shutdown"})
}

pub fn finds_something() -> Fake {
    Fake::new(|asked, _| Ok(vec![finding(&asked.key, "from the extension")]))
}

pub fn saw(asked: &Asked) -> Vec<Diagnostic> {
    vec![finding(&asked.key, &format!("saw {}", asked.text.trim()))]
}

pub fn warnings(written: &[Value]) -> Vec<String> {
    written
        .iter()
        .filter(|message| message["method"] == "window/showMessage")
        .map(|message| {
            assert_eq!(message["params"]["type"], 2, "a warning");
            message["params"]["message"]
                .as_str()
                .expect("a message")
                .to_owned()
        })
        .collect()
}
