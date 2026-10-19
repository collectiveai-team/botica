//! What goes over the wire: the frames of the protocol, the paths of the URIs and the JSON of the
//! findings.

use std::fmt::Write as _;
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

/// The `file://` URI of `path`, given from `root` or whole.
pub fn uri_of(path: &str, root: &Path) -> String {
    let whole = root.join(path);
    let mut uri = String::from("file://");
    for byte in whole.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            let _ = write!(uri, "%{byte:02X}");
        }
    }
    uri
}

/// How the columns of the protocol count: in UTF-16 code units, the default of the protocol, or
/// in bytes when the editor offers UTF-8.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16,
}

impl Encoding {
    /// The encoding of `positionEncodings`, the list the editor offers in its capabilities.
    pub fn chosen(offered: &Value) -> Self {
        let utf8 = offered
            .as_array()
            .is_some_and(|offered| offered.iter().any(|name| name == "utf-8"));
        if utf8 {
            Self::Utf8
        } else {
            Self::Utf16
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16 => "utf-16",
        }
    }

    const fn width(self, character: char) -> usize {
        match self {
            Self::Utf8 => character.len_utf8(),
            Self::Utf16 => character.len_utf16(),
        }
    }
}

/// The byte offset in `text` of a position of the protocol. A line past the end is the end of the
/// text, and a column past the end of its line is the end of the line.
pub fn offset_of(text: &str, position: &Value, encoding: Encoding) -> usize {
    let line = position["line"].as_u64().unwrap_or_default();
    let column =
        usize::try_from(position["character"].as_u64().unwrap_or_default()).unwrap_or(usize::MAX);
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(newline) => start += newline + 1,
            None => return text.len(),
        }
    }
    let end = text[start..]
        .find('\n')
        .map_or(text.len(), |newline| start + newline);
    let mut units = 0;
    for (index, character) in text[start..end].char_indices() {
        if units >= column {
            return start + index;
        }
        units += encoding.width(character);
    }
    end
}

/// Where the columns of the findings are counted from: the text they were found in, if the server
/// holds it, and the encoding of the session.
#[derive(Clone, Copy)]
pub struct Columns<'a> {
    pub text: Option<&'a str>,
    pub encoding: Encoding,
}

impl Columns<'_> {
    /// The column of the protocol of the 1-based `column`, counted in characters, of `line`.
    fn character(self, line: usize, column: usize) -> usize {
        let characters = column.saturating_sub(1);
        let Some(text) = self.text else {
            return characters;
        };
        let held = text.split('\n').nth(line.saturating_sub(1)).unwrap_or("");
        let mut units = 0;
        let mut counted = 0;
        for character in held.chars().take(characters) {
            units += self.encoding.width(character);
            counted += 1;
        }
        units + (characters - counted)
    }

    fn position(self, line: usize, column: usize) -> Value {
        json!({"line": line.saturating_sub(1), "character": self.character(line, column)})
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

fn diagnostic(found: &Diagnostic, columns: Columns<'_>) -> Value {
    let (start, end) = if found.file_level {
        (columns.position(1, 1), columns.position(1, 1))
    } else {
        (
            columns.position(found.line, found.column),
            columns.position(found.end_line, found.end_column),
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

pub fn publish(
    output: &mut impl Write,
    uri: &str,
    findings: &[Diagnostic],
    columns: Columns<'_>,
) -> io::Result<()> {
    let diagnostics: Vec<Value> = findings
        .iter()
        .map(|found| diagnostic(found, columns))
        .collect();
    write_message(
        output,
        &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                "params": {"uri": uri, "diagnostics": diagnostics}}),
    )
}

pub fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}
