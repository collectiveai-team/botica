//! Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.
//!
//! An extension registers rules with [`Rule::new`]; each rule receives `(path, text)` and returns
//! its [`Finding`]s, with `start` and `end` as byte offsets of the text. [`serve`] answers
//! `initialize`, `lint/files` and `shutdown`.

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};

pub const PROTOCOL_VERSION: u32 = 1;

/// One finding: `start` and `end` are byte offsets in the UTF-8 text of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

impl Finding {
    pub fn new(start: usize, end: usize, message: impl Into<String>) -> Self {
        Self {
            start,
            end,
            message: message.into(),
        }
    }
}

type Check = Box<dyn Fn(&str, &str) -> Vec<Finding>>;

pub struct Rule {
    id: String,
    languages: Vec<String>,
    severity: String,
    check: Check,
}

impl Rule {
    /// A rule that looks at the files of `languages` and has the severity `warning`.
    pub fn new(
        id: &str,
        languages: &[&str],
        check: impl Fn(&str, &str) -> Vec<Finding> + 'static,
    ) -> Self {
        Self {
            id: id.to_owned(),
            languages: languages
                .iter()
                .map(|language| (*language).to_owned())
                .collect(),
            severity: "warning".to_owned(),
            check: Box::new(check),
        }
    }

    /// `error`, `warning` or `info`.
    #[must_use]
    pub fn severity(mut self, severity: &str) -> Self {
        severity.clone_into(&mut self.severity);
        self
    }
}

fn language_of(path: &str) -> Option<&'static str> {
    match Path::new(path).extension()?.to_str()? {
        "py" => Some("python"),
        "ts" | "tsx" => Some("typescript"),
        "js" => Some("javascript"),
        _ => None,
    }
}

fn lint(rules: &[Rule], files: &[Value]) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    for file in files {
        let (Some(path), Some(text)) = (file["path"].as_str(), file["text"].as_str()) else {
            continue;
        };
        let language = language_of(path);
        for rule in rules {
            if !language.is_some_and(|language| rule.languages.iter().any(|l| l == language)) {
                continue;
            }
            for found in (rule.check)(path, text) {
                diagnostics.push(json!({
                    "path": path,
                    "rule": rule.id,
                    "severity": rule.severity,
                    "start": found.start,
                    "end": found.end,
                    "message": found.message,
                }));
            }
        }
    }
    diagnostics
}

fn answer(rules: &[Rule], request: &Value) -> Value {
    let method = request["method"].as_str().unwrap_or_default();
    let id = &request["id"];
    match method {
        "initialize" => {
            let listed: Vec<Value> = rules
                .iter()
                .map(|rule| json!({"id": rule.id, "languages": rule.languages}))
                .collect();
            json!({"jsonrpc": "2.0", "id": id, "result": {
                "protocolVersion": PROTOCOL_VERSION, "rules": listed,
            }})
        }
        "lint/files" => {
            let files = request["params"]["files"]
                .as_array()
                .map_or(&[][..], Vec::as_slice);
            json!({"jsonrpc": "2.0", "id": id, "result": {"diagnostics": lint(rules, files)}})
        }
        "shutdown" => json!({"jsonrpc": "2.0", "id": id, "result": null}),
        other => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": other}})
        }
    }
}

/// Serves the requests of `input` to the rules, one reply on `output` for each, until `shutdown`
/// or the end of `input`.
///
/// # Errors
/// Fails when `input` or `output` does, or when a line of `input` is not JSON.
pub fn serve_on(rules: &[Rule], input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    for line in input.lines() {
        let request: Value = serde_json::from_str(&line?)?;
        writeln!(output, "{}", answer(rules, &request))?;
        output.flush()?;
        if request["method"] == "shutdown" {
            break;
        }
    }
    Ok(())
}

/// Serves on stdin and stdout, and returns after `shutdown`.
///
/// # Errors
/// As [`serve_on`].
pub fn serve(rules: &[Rule]) -> io::Result<()> {
    serve_on(rules, io::stdin().lock(), io::stdout().lock())
}
