//! What goes over the wire: the frames of the protocol, the paths of the URIs and the JSON of the
//! findings.

use std::io::{self, BufRead, Write};
use std::path::Path;

use botica_core::{Diagnostic, Severity};
use serde_json::{json, Value};

pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<Value>> {
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

pub fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
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
pub fn path_of(uri: &str, root: &Path) -> String {
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

pub fn publish(output: &mut impl Write, uri: &str, findings: &[Diagnostic]) -> io::Result<()> {
    let diagnostics: Vec<Value> = findings.iter().map(diagnostic).collect();
    write_message(
        output,
        &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                "params": {"uri": uri, "diagnostics": diagnostics}}),
    )
}

pub fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}
