//! An editor that talks to `botica server` over its stdin and stdout. What the server writes is
//! read by a thread, so a read that gets nothing fails after a deadline instead of waiting.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const DEADLINE: Duration = Duration::from_secs(20);

pub struct Session {
    child: Child,
    stdin: ChildStdin,
    frames: Receiver<Value>,
}

fn read_frame(output: &mut impl BufRead) -> Option<Value> {
    let mut length = 0;
    loop {
        let mut line = String::new();
        if output.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        length = line
            .strip_prefix("Content-Length: ")
            .map_or(length, |value| value.parse().expect("a number"));
    }
    let mut body = vec![0; length];
    output.read_exact(&mut body).ok()?;
    Some(serde_json::from_slice(&body).expect("JSON"))
}

impl Session {
    /// Starts `botica` with `args` in `dir`.
    pub fn start(dir: &Path, args: &[&str]) -> Self {
        Self::start_with_env(dir, args, &[])
    }

    /// Starts `botica` with `args` in `dir` and the variables `env` added to its environment.
    pub fn start_with_env(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_botica"))
            .args(args)
            .envs(env.iter().copied())
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start botica");
        let stdin = child.stdin.take().expect("stdin");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let (sender, frames) = channel();
        std::thread::spawn(move || {
            while let Some(frame) = read_frame(&mut stdout) {
                if sender.send(frame).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            frames,
        }
    }

    pub fn send(&mut self, message: &Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).expect("write");
        self.stdin.flush().expect("flush");
    }

    /// The next message of the server, or a failure after the deadline.
    pub fn read(&mut self) -> Value {
        self.frames
            .recv_timeout(DEADLINE)
            .expect("the server wrote a message")
    }

    /// Asks the server to shut down and exit, returns the replies it still wrote, and how it
    /// ended.
    pub fn finish(mut self) -> (Vec<Value>, ExitStatus) {
        self.send(&json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown"}));
        let mut rest = Vec::new();
        loop {
            let message = self.read();
            let done = message["id"] == 99;
            rest.push(message);
            if done {
                break;
            }
        }
        self.send(&json!({"jsonrpc": "2.0", "method": "exit"}));
        let limit = Instant::now() + DEADLINE;
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("wait") {
                break status;
            }
            assert!(Instant::now() < limit, "botica did not end after `exit`");
            std::thread::sleep(Duration::from_millis(10));
        };
        (rest, status)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub fn initialize(options: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
           "params": {"capabilities": {}, "initializationOptions": options}})
}

pub fn open(uri: &str, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": uri, "languageId": "python", "version": 1, "text": text}}})
}
