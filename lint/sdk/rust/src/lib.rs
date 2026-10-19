//! Minimal botica extension SDK: NDJSON JSON-RPC 2.0 over stdio.
//!
//! An extension registers rules with [`Rule::new`]; each rule receives `(path, text, options)` and
//! returns its [`Finding`]s, with `start` and `end` as byte offsets of the text. A rule registered
//! with [`Rule::project`] receives `(root, paths, options)`, reads the files itself and returns
//! findings made with [`Finding::in_file`]. `options` holds the options that the configuration of
//! the project gives the rule. [`serve`] answers `initialize`, `lint/files`, `lint/project` and
//! `shutdown`, and runs the rules that each request names.

use std::io::{self, BufRead, Write};
use std::path::Path;

use serde_json::{json, Value};

pub const PROTOCOL_VERSION: u32 = 1;

/// One finding: `start` and `end` are byte offsets in the UTF-8 text of the file. `path` is the
/// file of a finding of a rule of project scope, and `None` for a rule of file scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: Option<String>,
    pub start: usize,
    pub end: usize,
    pub message: String,
}

impl Finding {
    /// A finding in the file that the rule of file scope is checking.
    pub fn new(start: usize, end: usize, message: impl Into<String>) -> Self {
        Self {
            path: None,
            start,
            end,
            message: message.into(),
        }
    }

    /// A finding in the file at `path`, for a rule of project scope.
    pub fn in_file(
        path: impl Into<String>,
        start: usize,
        end: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: Some(path.into()),
            ..Self::new(start, end, message)
        }
    }
}

type FileCheck = Box<dyn Fn(&str, &str, &Value) -> Vec<Finding>>;
type ProjectCheck = Box<dyn Fn(&str, &[String], &Value) -> Vec<Finding>>;

enum Check {
    File(FileCheck),
    Project(ProjectCheck),
}

pub struct Rule {
    id: String,
    languages: Vec<String>,
    severity: String,
    check: Check,
}

impl Rule {
    /// A rule of file scope that looks at the files of `languages` and has the severity `warning`.
    pub fn new(
        id: &str,
        languages: &[&str],
        check: impl Fn(&str, &str, &Value) -> Vec<Finding> + 'static,
    ) -> Self {
        Self::with(id, languages, Check::File(Box::new(check)))
    }

    /// A rule of project scope with the severity `warning`. A finding of it without a path is
    /// skipped.
    pub fn project(
        id: &str,
        languages: &[&str],
        check: impl Fn(&str, &[String], &Value) -> Vec<Finding> + 'static,
    ) -> Self {
        Self::with(id, languages, Check::Project(Box::new(check)))
    }

    fn with(id: &str, languages: &[&str], check: Check) -> Self {
        Self {
            id: id.to_owned(),
            languages: languages
                .iter()
                .map(|language| (*language).to_owned())
                .collect(),
            severity: "warning".to_owned(),
            check,
        }
    }

    /// `error`, `warning` or `info`.
    #[must_use]
    pub fn severity(mut self, severity: &str) -> Self {
        severity.clone_into(&mut self.severity);
        self
    }

    fn diagnostic(&self, path: &str, found: Finding) -> Value {
        json!({
            "path": path,
            "rule": self.id,
            "severity": self.severity,
            "start": found.start,
            "end": found.end,
            "message": found.message,
        })
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

/// The rules that the request names (every one when it names none), each with its options: an
/// empty object when the request gives it none.
fn named<'a>(rules: &'a [Rule], params: &Value) -> Vec<(&'a Rule, Value)> {
    let names = params["rules"].as_array();
    rules
        .iter()
        .filter(|rule| names.is_none_or(|names| names.iter().any(|name| *name == rule.id)))
        .map(|rule| {
            let options = match &params["options"][&rule.id] {
                Value::Null => json!({}),
                options => options.clone(),
            };
            (rule, options)
        })
        .collect()
}

fn lint(rules: &[Rule], params: &Value) -> Vec<Value> {
    let named = named(rules, params);
    let files = params["files"].as_array().map_or(&[][..], Vec::as_slice);
    let mut diagnostics = Vec::new();
    for file in files {
        let (Some(path), Some(text)) = (file["path"].as_str(), file["text"].as_str()) else {
            continue;
        };
        let language = language_of(path);
        for (rule, options) in &named {
            let Check::File(check) = &rule.check else {
                continue;
            };
            if !language.is_some_and(|language| rule.languages.iter().any(|l| l == language)) {
                continue;
            }
            for found in check(path, text, options) {
                diagnostics.push(rule.diagnostic(path, found));
            }
        }
    }
    diagnostics
}

fn lint_project(rules: &[Rule], params: &Value) -> Vec<Value> {
    let root = params["root"].as_str().unwrap_or_default();
    let paths: Vec<String> = params["files"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|path| path.as_str().map(str::to_owned))
        .collect();
    let mut diagnostics = Vec::new();
    for (rule, options) in named(rules, params) {
        let Check::Project(check) = &rule.check else {
            continue;
        };
        for found in check(root, &paths, &options) {
            if let Some(path) = found.path.clone() {
                diagnostics.push(rule.diagnostic(&path, found));
            }
        }
    }
    diagnostics
}

fn answer(rules: &[Rule], request: &Value) -> Value {
    let method = request["method"].as_str().unwrap_or_default();
    let id = &request["id"];
    let params = &request["params"];
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
            json!({"jsonrpc": "2.0", "id": id, "result": {"diagnostics": lint(rules, params)}})
        }
        "lint/project" => {
            let diagnostics = lint_project(rules, params);
            json!({"jsonrpc": "2.0", "id": id, "result": {"diagnostics": diagnostics}})
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
