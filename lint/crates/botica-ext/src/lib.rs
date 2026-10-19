//! Extension host: JSON-RPC 2.0 over stdio, one JSON message per line.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: u32 = 1;

/// How long an extension has to end after `shutdown` before it is killed.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// How long an extension has to answer `initialize`: a script run with `uv` may download its
/// dependencies first.
const INITIALIZE_DEADLINE: Duration = Duration::from_secs(120);

/// How long an extension has to answer `lint/files` and `shutdown`.
const REQUEST_DEADLINE: Duration = Duration::from_secs(60);

/// The environment variable that sets, in seconds, the deadline of `initialize` and of
/// `lint/files` in place of the two above.
pub const DEADLINE_VARIABLE: &str = "BOTICA_EXTENSION_TIMEOUT";

#[derive(Debug, Serialize)]
pub struct FileIn<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WireDiagnostic {
    pub path: String,
    pub rule: String,
    pub severity: String,
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// The rules that a request asks for, and the options of each one, by id.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Selection {
    pub rules: Vec<String>,
    pub options: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct RuleInfo {
    pub id: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct InitializeResult {
    rules: Vec<RuleInfo>,
}

#[derive(Debug, Deserialize)]
struct LintResult {
    diagnostics: Vec<WireDiagnostic>,
}

pub struct ExtensionHost {
    child: Child,
    requests: Sender<Vec<u8>>,
    replies: Receiver<String>,
    deadlines: Deadlines,
    next_id: u64,
    pub rules: Vec<RuleInfo>,
}

/// How long an extension has to answer each request before it is killed.
#[derive(Debug, Clone, Copy)]
pub struct Deadlines {
    pub initialize: Duration,
    pub request: Duration,
}

impl Deadlines {
    /// The defaults, or `DEADLINE_VARIABLE` seconds for both when it holds a number.
    pub fn from_env() -> Self {
        match std::env::var(DEADLINE_VARIABLE)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            Some(seconds) => Self {
                initialize: Duration::from_secs(seconds),
                request: Duration::from_secs(seconds),
            },
            None => Self {
                initialize: INITIALIZE_DEADLINE,
                request: REQUEST_DEADLINE,
            },
        }
    }
}

/// Writes each line it is sent to `stdin`; it ends when the process stops reading or the host is
/// dropped. A write that blocks on a full pipe blocks this thread, not `call`.
fn write_requests(mut stdin: impl Write + Send + 'static) -> Sender<Vec<u8>> {
    let (sender, lines) = channel::<Vec<u8>>();
    std::thread::spawn(move || {
        for line in lines {
            if stdin.write_all(&line).and_then(|()| stdin.flush()).is_err() {
                break;
            }
        }
    });
    sender
}

/// Sends each line of `stdout`; it ends, and so does the channel, when the process closes it.
fn read_replies(stdout: impl BufRead + Send + 'static) -> Receiver<String> {
    let (sender, replies) = channel();
    std::thread::spawn(move || {
        for line in stdout.lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    replies
}

impl ExtensionHost {
    pub fn spawn(command: &[String]) -> Result<Self> {
        Self::spawn_in(command, None)
    }

    /// Starts the extension with `dir` as its working directory (botica's own when `None`).
    pub fn spawn_in(command: &[String], dir: Option<&Path>) -> Result<Self> {
        Self::spawn_with(command, dir, Deadlines::from_env())
    }

    pub fn spawn_with(
        command: &[String],
        dir: Option<&Path>,
        deadlines: Deadlines,
    ) -> Result<Self> {
        let (program, args) = command.split_first().context("empty extension command")?;
        let mut process = Command::new(program);
        process.args(args);
        if let Some(dir) = dir {
            process.current_dir(dir);
        }
        let mut child = process
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("cannot start extension `{program}`"))?;
        let stdin = child.stdin.take().context("extension stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("extension stdout")?);
        let mut host = Self {
            child,
            requests: write_requests(stdin),
            replies: read_replies(stdout),
            deadlines,
            next_id: 0,
            rules: Vec::new(),
        };
        let init: InitializeResult = host.call(
            "initialize",
            json!({ "protocolVersion": PROTOCOL_VERSION, "client": "botica" }),
            deadlines.initialize,
        )?;
        host.rules = init.rules;
        Ok(host)
    }

    /// Sends the files with their texts; with a selection, it names the rules and their options.
    pub fn lint_files(
        &mut self,
        files: &[FileIn<'_>],
        selection: Option<&Selection>,
    ) -> Result<Vec<WireDiagnostic>> {
        let mut params = json!({ "files": files });
        if let Some(selection) = selection {
            params["rules"] = json!(selection.rules);
            params["options"] = Value::Object(selection.options.clone());
        }
        let result: LintResult = self.call("lint/files", params, self.deadlines.request)?;
        Ok(result.diagnostics)
    }

    /// Sends the root of the project and the paths of its files, for the rules of project scope:
    /// the extension reads the files itself.
    pub fn lint_project(
        &mut self,
        root: &str,
        files: &[&str],
        selection: &Selection,
    ) -> Result<Vec<WireDiagnostic>> {
        let params = json!({
            "root": root,
            "files": files,
            "rules": selection.rules,
            "options": selection.options,
        });
        let result: LintResult = self.call("lint/project", params, self.deadlines.request)?;
        Ok(result.diagnostics)
    }

    /// Fails unless the extension declared, when it started, every rule of `required` and no rule
    /// outside `allowed`: the rules that the catalogs run with the same program.
    pub fn check_declared(&self, required: &[String], allowed: &[String]) -> Result<()> {
        let declared: Vec<&str> = self.rules.iter().map(|rule| rule.id.as_str()).collect();
        let missing: Vec<String> = required
            .iter()
            .filter(|id| !declared.contains(&id.as_str()))
            .map(|id| format!("`{id}`"))
            .collect();
        let extra: Vec<String> = declared
            .iter()
            .filter(|id| !allowed.iter().chain(required).any(|known| known == *id))
            .map(|id| format!("`{id}`"))
            .collect();
        let mut problems = Vec::new();
        if !missing.is_empty() {
            problems.push(format!("does not declare {}", missing.join(", ")));
        }
        if !extra.is_empty() {
            problems.push(format!(
                "declares {}, which its catalog does not run with it",
                extra.join(", ")
            ));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            bail!(
                "the extension {} in its answer to `initialize`",
                problems.join(" and ")
            )
        }
    }

    /// Asks the extension to end and waits `SHUTDOWN_GRACE` for it to do so; a process that is
    /// still running then is killed and the shutdown is an error.
    pub fn shutdown(mut self) -> Result<()> {
        let _: Value = self.call("shutdown", Value::Null, self.deadlines.request)?;
        let deadline = Instant::now() + SHUTDOWN_GRACE;
        while self.child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                self.child.kill()?;
                self.child.wait()?;
                bail!(
                    "extension did not end within {} seconds of `shutdown` and was killed",
                    SHUTDOWN_GRACE.as_secs()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    /// Sends the request and waits `deadline` for the reply; a process that has not answered by
    /// then is killed.
    fn call<T: for<'de> Deserialize<'de>>(
        &mut self,
        method: &str,
        params: Value,
        deadline: Duration,
    ) -> Result<T> {
        self.next_id += 1;
        let request =
            json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params });
        let mut line = serde_json::to_vec(&request)?;
        line.push(b'\n');
        // A closed channel means the process stopped reading; the reply channel says so below.
        let _ = self.requests.send(line);

        let response = match self.replies.recv_timeout(deadline) {
            Ok(response) => response,
            Err(RecvTimeoutError::Timeout) => {
                self.child.kill()?;
                self.child.wait()?;
                bail!("extension did not answer `{method}` within {deadline:?} and was killed");
            }
            Err(RecvTimeoutError::Disconnected) => {
                bail!("extension closed its stdout during `{method}`")
            }
        };
        let mut response: Value = serde_json::from_str(&response)?;
        if let Some(error) = response.get("error") {
            bail!("extension error on `{method}`: {error}");
        }
        let result = response
            .get_mut("result")
            .map(Value::take)
            .ok_or_else(|| anyhow!("extension reply to `{method}` has no result"))?;
        Ok(serde_json::from_value(result)?)
    }
}

impl Drop for ExtensionHost {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
