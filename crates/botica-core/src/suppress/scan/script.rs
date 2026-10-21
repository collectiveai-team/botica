//! The lexer of JavaScript and TypeScript: comments, strings, template literals and regex
//! literals.

use super::{block_comment_end, is_word, line_end, string_end, Scan};

enum Frame {
    Template { start: usize },
    Interpolation { depth: u32 },
}

const REGEX_AFTER: [&[u8]; 12] = [
    b"return",
    b"typeof",
    b"instanceof",
    b"in",
    b"of",
    b"new",
    b"delete",
    b"void",
    b"throw",
    b"case",
    b"do",
    b"else",
];

/// Whether a `/` after the last significant byte `last` starts a regex literal and not a division.
fn regex_may_start(bytes: &[u8], last: Option<usize>) -> bool {
    let Some(last) = last else {
        return true;
    };
    let byte = bytes[last];
    if is_word(byte) {
        let mut start = last;
        while start > 0 && is_word(bytes[start - 1]) {
            start -= 1;
        }
        return REGEX_AFTER.contains(&&bytes[start..=last]);
    }
    !matches!(byte, b')' | b']' | b'}')
}

/// End of the regex literal that opens at `at`, or `None` when the line ends first.
fn regex_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut j = at + 1;
    let mut in_class = false;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => j += 1,
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => return Some(j + 1),
            b'\n' => return None,
            _ => {}
        }
        j += 1;
    }
    None
}

pub(super) fn script(bytes: &[u8]) -> Scan {
    let mut lexer = Lexer {
        bytes,
        scan: Scan::default(),
        stack: Vec::new(),
        last: None,
        i: 0,
    };
    while lexer.i < bytes.len() {
        match lexer.stack.last() {
            Some(&Frame::Template { start }) => lexer.template_byte(start),
            _ => lexer.code_byte(),
        }
    }
    lexer.scan
}

/// The state of the lexer at byte `i`.
struct Lexer<'a> {
    bytes: &'a [u8],
    scan: Scan,
    /// The template literals and their interpolations the lexer is in, the innermost last.
    stack: Vec<Frame>,
    /// The last significant byte.
    last: Option<usize>,
    i: usize,
}

impl Lexer<'_> {
    /// Reads at byte `i` inside the text of the template literal that opens at `start`.
    fn template_byte(&mut self, start: usize) {
        let i = self.i;
        match self.bytes[i] {
            b'\\' => self.i += 2,
            b'`' => {
                self.stack.pop();
                if self.stack.is_empty() && self.bytes[start..=i].contains(&b'\n') {
                    self.scan.strings.push(start..i + 1);
                }
                self.last = Some(i);
                self.i += 1;
            }
            b'$' if self.bytes.get(i + 1) == Some(&b'{') => {
                self.stack.push(Frame::Interpolation { depth: 0 });
                self.i += 2;
            }
            _ => self.i += 1,
        }
    }

    /// Reads at byte `i` in code, outside the text of a template literal.
    fn code_byte(&mut self) {
        let (bytes, i) = (self.bytes, self.i);
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = line_end(bytes, i);
                self.scan.comments.push(i + 2..end);
                self.i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = block_comment_end(bytes, i);
                self.scan.comments.push(i + 2..end);
                self.i = (end + 2).min(bytes.len());
            }
            b'/' if regex_may_start(bytes, self.last) => self.slash(),
            quote @ (b'"' | b'\'') => {
                let end = string_end(bytes, i, quote, false).min(bytes.len());
                self.last = Some(end.saturating_sub(1).max(i));
                self.i = end.max(i + 1);
            }
            b'`' => {
                self.stack.push(Frame::Template { start: i });
                self.i += 1;
            }
            b'{' => self.open_brace(),
            b'}' => self.close_brace(),
            byte => {
                if !byte.is_ascii_whitespace() {
                    self.last = Some(i);
                }
                self.i += 1;
            }
        }
    }

    /// A `/` where a regex literal may start: the literal, or a lone `/` when the line ends first.
    fn slash(&mut self) {
        match regex_end(self.bytes, self.i) {
            Some(end) => {
                self.last = Some(end - 1);
                self.i = end;
            }
            None => {
                self.last = Some(self.i);
                self.i += 1;
            }
        }
    }

    fn open_brace(&mut self) {
        if let Some(Frame::Interpolation { depth }) = self.stack.last_mut() {
            *depth += 1;
        }
        self.last = Some(self.i);
        self.i += 1;
    }

    /// A `}`, which closes the interpolation it is in when it opened none of its own.
    fn close_brace(&mut self) {
        if let Some(Frame::Interpolation { depth }) = self.stack.last_mut() {
            if *depth == 0 {
                self.stack.pop();
            } else {
                *depth -= 1;
            }
        }
        self.last = Some(self.i);
        self.i += 1;
    }
}
