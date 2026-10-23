//! Finds the real comments of a file (not the text of a string) and the strings that span lines.
//!
//! Each language family gets a small lexer that knows its comments and its strings, nothing else.
//! Every delimiter is ASCII, so the ranges fall on `char` boundaries of a valid `&str`.

mod script;

use script::script;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Syntax {
    /// `#` comments and `'`, `"`, `'''`, `"""` strings: Python and TOML, `.cargo/config` too.
    PyLike,
    /// `#` comments after a blank, and one-line `'` and `"` strings: shell, YAML, Dockerfile, Makefile.
    Hash,
    /// `//` and `/* */` comments, strings, template literals and regex literals: JavaScript and TypeScript.
    Script,
    /// `<!-- -->` comments: HTML and Markdown.
    Markup,
    /// `//` and `/* */` comments, `"` strings, raw strings and character literals: Go and Rust.
    CLike,
    /// `#` and `;` comments on a line of their own: INI files as pytest reads them, where a `#`
    /// after a value is part of the value.
    Ini,
    /// `#` and `;` comments on a line of their own or after a value that is not quoted: `.npmrc`,
    /// as npm reads it.
    Npmrc,
}

impl Syntax {
    pub(crate) fn of(path: &str) -> Option<Self> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let extension = name.rsplit_once('.').map(|(_, extension)| extension);
        match extension {
            Some("py" | "pyi" | "toml") => Some(Self::PyLike),
            Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts") => Some(Self::Script),
            Some(
                "sh" | "bash" | "zsh" | "yml" | "yaml" | "mk" | "dockerfile" | "cfg" | "coveragerc",
            ) => Some(Self::Hash),
            Some("md" | "markdown" | "html" | "htm") => Some(Self::Markup),
            Some("go" | "rs") => Some(Self::CLike),
            Some("ini") => Some(Self::Ini),
            _ if name == ".npmrc" => Some(Self::Npmrc),
            _ if path == ".cargo/config" || path.ends_with("/.cargo/config") => Some(Self::PyLike),
            _ if name == "Dockerfile" || name.starts_with("Dockerfile.") || name == "Makefile" => {
                Some(Self::Hash)
            }
            _ => None,
        }
    }
}

/// The `noqa` channel exists only in Python.
pub(crate) fn allows_noqa(path: &str) -> bool {
    path.ends_with(".py") || path.ends_with(".pyi")
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Scan {
    /// The text of each comment, without its delimiters.
    pub comments: Vec<Range<usize>>,
    /// Strings that contain a newline, delimiters included.
    pub strings: Vec<Range<usize>>,
}

pub(crate) fn scan(text: &str, syntax: Syntax) -> Scan {
    let bytes = text.as_bytes();
    match syntax {
        Syntax::PyLike => hash_family(bytes, true, false),
        Syntax::Hash => hash_family(bytes, false, true),
        Syntax::Script => script(bytes),
        Syntax::Markup => markup(text),
        Syntax::CLike => c_like(bytes),
        Syntax::Ini => ini(text, false),
        Syntax::Npmrc => ini(text, true),
    }
}

/// The comments of an INI file: a line whose first character that is not blank is `#` or `;`,
/// and with `after_values`, what follows the first `#` or `;` of a value that is not quoted, as
/// npm reads `.npmrc`.
fn ini(text: &str, after_values: bool) -> Scan {
    let mut scan = Scan::default();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let lead = body.len() - body.trim_start().len();
        let at = match body.as_bytes().get(lead) {
            Some(b'#' | b';') => Some(lead),
            _ if after_values => npmrc_comment(body),
            _ => None,
        };
        if let Some(at) = at {
            scan.comments.push(start + at + 1..start + body.len());
        }
        start += line.len();
    }
    scan
}

/// Where the comment of a line of `.npmrc` starts: npm cuts a value that is not quoted at its
/// first `#` or `;` that a `\` does not escape.
fn npmrc_comment(line: &str) -> Option<usize> {
    let from = line.find('=').map_or(0, |at| at + 1);
    let value = line[from..].trim();
    let quoted = value.len() > 1
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')));
    if quoted {
        return None;
    }
    let bytes = line.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'#' | b';' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Where a block comment that opens at `at` ends (after its `*/`).
fn block_comment_end(bytes: &[u8], at: usize) -> usize {
    bytes[at + 2..]
        .windows(2)
        .position(|pair| pair == b"*/")
        .map_or(bytes.len(), |end| at + 2 + end)
}

/// Where the string that opens at `at` with `"` ends, newlines included: Go and Rust.
fn quoted_end(bytes: &[u8], at: usize) -> usize {
    let mut j = at + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            _ => j += 1,
        }
    }
    bytes.len()
}

/// Where a raw string of Rust that opens at `at` (`r"`, `r#"`, `br##"`…) ends, or `None` when
/// none opens there.
fn raw_string_end(bytes: &[u8], at: usize) -> Option<usize> {
    if at > 0 && is_word(bytes[at - 1]) {
        return None;
    }
    let mut j = at + usize::from(bytes[at] == b'b');
    if bytes.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let hashes = bytes[j..].iter().take_while(|&&byte| byte == b'#').count();
    j += hashes;
    if bytes.get(j) != Some(&b'"') {
        return None;
    }
    let mut close = vec![b'"'];
    close.extend(std::iter::repeat_n(b'#', hashes));
    Some(
        bytes[j + 1..]
            .windows(close.len())
            .position(|window| window == close.as_slice())
            .map_or(bytes.len(), |end| j + 1 + end + close.len()),
    )
}

/// Where a character literal that opens at `at` ends, or `None` for a lifetime of Rust (`'a`).
fn char_end(bytes: &[u8], at: usize) -> Option<usize> {
    match bytes.get(at + 1)? {
        b'\\' => {
            let close = bytes[at + 2..].iter().position(|&byte| byte == b'\'')?;
            Some(at + 2 + close + 1)
        }
        &first => {
            let width = match first {
                0xF0.. => 4,
                0xE0.. => 3,
                0xC0.. => 2,
                _ => 1,
            };
            (bytes.get(at + 1 + width) == Some(&b'\'')).then_some(at + 2 + width)
        }
    }
}

fn c_like(bytes: &[u8]) -> Scan {
    let mut scan = Scan::default();
    let mut i = 0;
    while i < bytes.len() {
        let end = match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = line_end(bytes, i);
                scan.comments.push(i + 2..end);
                i = end;
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = block_comment_end(bytes, i);
                scan.comments.push(i + 2..end);
                i = (end + 2).min(bytes.len());
                continue;
            }
            b'r' | b'b' => raw_string_end(bytes, i),
            b'"' => Some(quoted_end(bytes, i)),
            b'`' => Some(
                bytes[i + 1..]
                    .iter()
                    .position(|&byte| byte == b'`')
                    .map_or(bytes.len(), |end| i + 1 + end + 1),
            ),
            b'\'' => char_end(bytes, i),
            _ => None,
        };
        match end {
            Some(end) => {
                if bytes[i..end].contains(&b'\n') {
                    scan.strings.push(i..end);
                }
                i = end.max(i + 1);
            }
            None => i += 1,
        }
    }
    scan
}

fn line_end(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|&byte| byte == b'\n')
        .map_or(bytes.len(), |at| from + at)
}

/// Where the string that opens at `at` with `quote` ends (after its closing quote). One-line strings
/// stop before the newline when they are not closed.
fn string_end(bytes: &[u8], at: usize, quote: u8, triple: bool) -> usize {
    let len = bytes.len();
    if triple && bytes.get(at + 1) == Some(&quote) && bytes.get(at + 2) == Some(&quote) {
        let mut j = at + 3;
        while j < len {
            if bytes[j] == b'\\' {
                j += 2;
            } else if bytes[j] == quote
                && j + 2 < len
                && bytes[j + 1] == quote
                && bytes[j + 2] == quote
            {
                return j + 3;
            } else {
                j += 1;
            }
        }
        return len;
    }
    let mut j = at + 1;
    while j < len {
        match bytes[j] {
            b'\\' => j += 2,
            b'\n' => return j,
            byte if byte == quote => return j + 1,
            _ => j += 1,
        }
    }
    len
}

fn hash_family(bytes: &[u8], triple: bool, hash_needs_blank: bool) -> Scan {
    let mut scan = Scan::default();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'#' if !hash_needs_blank || i == 0 || matches!(bytes[i - 1], b' ' | b'\t' | b'\n') => {
                let end = line_end(bytes, i);
                scan.comments.push(i + 1..end);
                i = end;
            }
            quote @ (b'"' | b'\'') => {
                let end = string_end(bytes, i, quote, triple).min(bytes.len());
                if bytes[i..end].contains(&b'\n') {
                    scan.strings.push(i..end);
                }
                i = end.max(i + 1);
            }
            _ => i += 1,
        }
    }
    scan
}

fn markup(text: &str) -> Scan {
    let mut scan = Scan::default();
    let mut from = 0;
    while let Some(at) = text[from..].find("<!--") {
        let body = from + at + 4;
        match text[body..].find("-->") {
            Some(close) => {
                scan.comments.push(body..body + close);
                from = body + close + 3;
            }
            None => {
                scan.comments.push(body..text.len());
                break;
            }
        }
    }
    scan
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}
