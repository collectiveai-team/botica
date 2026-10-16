//! The Language Server Protocol front-end of botica: the findings of the rules for the buffers
//! of an editor, framed as the protocol frames them (`Content-Length` headers, JSON-RPC 2.0).
//!
//! The server holds no documents: each `didOpen` and `didChange` lints the text the editor sends
//! and publishes the findings, and `didClose` clears them. Columns are counted in characters,
//! which are the units of the protocol for text in the Basic Multilingual Plane.

use std::io::{self, BufRead, Write};
use std::path::Path;

use botica_core::{Diagnostic, Severity};
use serde_json::{json, Value};

/// What lints a document: its path from the root of the project, and its text.
pub type Lint<'a> = &'a dyn Fn(&str, &str) -> Vec<Diagnostic>;

fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "a message without Content-Length",
        ));
    };
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = message.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                index += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// The path of a `file://` URI, from `root` when it is inside it.
fn path_of(uri: &str, root: &Path) -> String {
    let path = percent_decode(uri.strip_prefix("file://").unwrap_or(uri));
    match Path::new(&path).strip_prefix(root) {
        Ok(inside) => inside.to_string_lossy().into_owned(),
        Err(_) => path,
    }
}

const fn level(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
        Severity::Hint => 4,
    }
}

fn position(line: usize, column: usize) -> Value {
    json!({"line": line.saturating_sub(1), "character": column.saturating_sub(1)})
}

fn diagnostic(found: &Diagnostic) -> Value {
    let (start, end) = if found.file_level {
        (position(1, 1), position(1, 1))
    } else {
        (
            position(found.line, found.column),
            position(found.end_line, found.end_column),
        )
    };
    json!({
        "range": {"start": start, "end": end},
        "severity": level(found.severity),
        "code": found.rule,
        "source": "botica",
        "message": found.message,
    })
}

fn publish(output: &mut impl Write, uri: &str, findings: &[Diagnostic]) -> io::Result<()> {
    let diagnostics: Vec<Value> = findings.iter().map(diagnostic).collect();
    write_message(
        output,
        &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                "params": {"uri": uri, "diagnostics": diagnostics}}),
    )
}

fn lint_and_publish(
    output: &mut impl Write,
    root: &Path,
    lint: Lint<'_>,
    uri: &str,
    text: &str,
) -> io::Result<()> {
    publish(output, uri, &lint(&path_of(uri, root), text))
}

fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Serves the messages of `input` until `exit` or the end of `input`. `lint` lints a document by
/// its path from `root`.
///
/// # Errors
/// Fails when `input` or `output` does, or when a message is not framed or is not JSON.
pub fn serve_on(
    mut input: impl BufRead,
    mut output: impl Write,
    root: &Path,
    lint: Lint<'_>,
) -> io::Result<()> {
    while let Some(message) = read_message(&mut input)? {
        let method = message["method"].as_str().unwrap_or_default();
        let params = &message["params"];
        let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
        match method {
            "exit" => return Ok(()),
            "initialize" => write_message(
                &mut output,
                &reply(
                    &message["id"],
                    json!({
                        "capabilities": {"textDocumentSync": {"openClose": true, "change": 1}},
                        "serverInfo": {"name": "botica"},
                    }),
                ),
            )?,
            "shutdown" => write_message(&mut output, &reply(&message["id"], Value::Null))?,
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or_default();
                lint_and_publish(&mut output, root, lint, uri, text)?;
            }
            "textDocument/didChange" => {
                let changes = params["contentChanges"].as_array();
                if let Some(text) = changes
                    .and_then(|changes| changes.last())
                    .and_then(|change| change["text"].as_str())
                {
                    lint_and_publish(&mut output, root, lint, uri, text)?;
                }
            }
            "textDocument/didClose" => publish(&mut output, uri, &[])?,
            _ if message.get("id").is_some() => write_message(
                &mut output,
                &json!({"jsonrpc": "2.0", "id": message["id"],
                        "error": {"code": -32601, "message": method}}),
            )?,
            _ => {}
        }
    }
    Ok(())
}

/// Serves on stdin and stdout.
///
/// # Errors
/// As [`serve_on`].
pub fn serve(root: &Path, lint: Lint<'_>) -> io::Result<()> {
    serve_on(io::stdin().lock(), io::stdout().lock(), root, lint)
}
