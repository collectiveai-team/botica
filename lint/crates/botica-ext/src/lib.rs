//! Extension host: JSON-RPC 2.0 over stdio, one JSON message per line.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub const PROTOCOL_VERSION: u32 = 1;

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
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    pub rules: Vec<RuleInfo>,
}

impl ExtensionHost {
    pub fn spawn(command: &[String]) -> Result<Self> {
        let (program, args) = command.split_first().context("empty extension command")?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("cannot start extension `{program}`"))?;
        let stdin = child.stdin.take().context("extension stdin")?;
        let stdout = BufReader::new(child.stdout.take().context("extension stdout")?);
        let mut host = Self {
            child,
            stdin,
            stdout,
            next_id: 0,
            rules: Vec::new(),
        };
        let init: InitializeResult = host.call(
            "initialize",
            json!({ "protocolVersion": PROTOCOL_VERSION, "client": "botica-spike" }),
        )?;
        host.rules = init.rules;
        Ok(host)
    }

    pub fn lint_files(&mut self, files: &[FileIn<'_>]) -> Result<Vec<WireDiagnostic>> {
        let result: LintResult = self.call("lint/files", json!({ "files": files }))?;
        Ok(result.diagnostics)
    }

    pub fn shutdown(mut self) -> Result<()> {
        let _: Value = self.call("shutdown", Value::Null)?;
        self.child.wait()?;
        Ok(())
    }

    fn call<T: for<'de> Deserialize<'de>>(&mut self, method: &str, params: Value) -> Result<T> {
        self.next_id += 1;
        let request =
            json!({ "jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params });
        let mut line = serde_json::to_vec(&request)?;
        line.push(b'\n');
        self.stdin.write_all(&line)?;
        self.stdin.flush()?;

        let mut response = String::new();
        if self.stdout.read_line(&mut response)? == 0 {
            bail!("extension closed its stdout during `{method}`");
        }
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
