use super::{build, merge, narrow, Chunk, Split};

/// Splits Python source into top-level statements. Comment-only and blank lines at column 0 stay
/// with the statement before them; `else`/`elif`/`except`/`finally` and decorated definitions
/// stay with the statement they continue.
pub(super) fn python_chunks(text: &[u8]) -> Split {
    python_split(text, 0, text.len(), 0, 0)
}

/// Bytes the Python lexer must look at outside strings; everything else is skipped in bulk.
const PY_INTERESTING: [bool; 256] = {
    let mut table = [false; 256];
    let bytes = b"#'\"()[]{}\\\n";
    let mut k = 0;
    while k < bytes.len() {
        table[bytes[k] as usize] = true;
        k += 1;
    }
    table
};

/// Splits `text[from..to]` into statements at indentation `level` (0 = top level). Indentation is
/// only checked for syntax errors at the top level.
fn python_split(text: &[u8], from: usize, to: usize, from_row: usize, level: usize) -> Split {
    let mut lexer = PyLexer::new(text, from, to, from_row, level);
    lexer.run();
    lexer.finish()
}

/// The state of `python_split` while it walks `text[from..to]`.
struct PyLexer<'a> {
    text: &'a [u8],
    from: usize,
    to: usize,
    level: usize,
    /// (start, row, body indent if the statement is a class/def)
    starts: Vec<(usize, usize, Option<usize>)>,
    head_seen: bool,
    compound: bool,
    depth: usize,
    /// (quote byte, triple)
    quote: Option<(u8, bool)>,
    continued: bool,
    after_decorator: bool,
    indents: Vec<usize>,
    /// The last significant code byte.
    last: Option<u8>,
    suspect: bool,
    row: usize,
    i: usize,
}

impl<'a> PyLexer<'a> {
    fn new(text: &'a [u8], from: usize, to: usize, from_row: usize, level: usize) -> Self {
        Self {
            text,
            from,
            to,
            level,
            starts: vec![(from, from_row, None)],
            head_seen: false,
            compound: false,
            depth: 0,
            quote: None,
            continued: false,
            after_decorator: false,
            indents: vec![0],
            last: None,
            suspect: false,
            row: from_row,
            i: from,
        }
    }

    fn run(&mut self) {
        let mut line_start = true;
        while self.i < self.to {
            if line_start {
                self.at_line_start();
            }
            let scanning = match self.quote {
                Some((quote, triple)) => self.in_string(quote, triple),
                None => self.in_code(),
            };
            if !scanning {
                break;
            }
            line_start = self.text.get(self.i) == Some(&b'\n');
            if line_start {
                self.row += 1;
            }
            self.i += 1;
        }
    }

    fn finish(self) -> Split {
        Split {
            chunks: build(&self.starts, self.to, self.row),
            suspect: self.suspect || self.depth != 0 || self.quote.is_some(),
        }
    }

    #[inline]
    fn at_line_start(&mut self) {
        if self.quote.is_none() && self.depth == 0 && !self.continued {
            self.logical_line();
        }
        self.continued = false;
    }

    /// Checks the indentation of a logical line and starts a statement at it if it may.
    fn logical_line(&mut self) {
        let (text, i, to) = (self.text, self.i, self.to);
        let indent = text[i..to]
            .iter()
            .take_while(|b| **b == b' ' || **b == b'\t')
            .count();
        let first = text.get(i + indent).copied().filter(|_| i + indent < to);
        if !first.is_some_and(|b| b != b'\n' && b != b'\r' && b != b'#') {
            return;
        }
        if self.level == 0 {
            self.suspect |= !indentation_ok(&mut self.indents, indent, self.last == Some(b':'));
        }
        let current = self.starts.len() - 1;
        if indent > self.level && self.compound && self.starts[current].2.is_none() {
            self.starts[current].2 = Some(indent);
        }
        if indent == self.level {
            self.statement_line(&text[i + indent..to]);
        }
    }

    /// A line at the indentation of the statements, starting with `rest`.
    fn statement_line(&mut self, rest: &[u8]) {
        let continues = ["else", "elif", "except", "finally"]
            .iter()
            .any(|kw| keyword_at(rest, kw.as_bytes()));
        if !continues && !self.after_decorator && self.i > self.from {
            self.starts.push((self.i, self.row, None));
            self.head_seen = false;
            self.compound = false;
        }
        self.after_decorator = rest[0] == b'@';
        if !self.after_decorator && !self.head_seen {
            self.head_seen = true;
            self.compound = keyword_at(rest, b"class")
                || keyword_at(rest, b"def")
                || keyword_at(rest, b"async");
        }
    }

    /// One step inside a string; `false` when the text ends in it.
    #[inline]
    fn in_string(&mut self, quote: u8, triple: bool) -> bool {
        let text = self.text;
        match memchr::memchr3(quote, b'\\', b'\n', &text[self.i..self.to]) {
            Some(offset) => self.i += offset,
            None => return false,
        }
        let byte = text[self.i];
        if byte == b'\\' {
            self.i += 1;
        } else if byte == quote {
            self.close_quote(quote, triple);
        } else if !triple {
            self.quote = None;
            self.suspect = true;
        }
        true
    }

    #[inline]
    fn close_quote(&mut self, quote: u8, triple: bool) {
        if !triple {
            self.quote = None;
            self.last = Some(quote);
        } else if self.text[self.i..].starts_with(&[quote, quote, quote]) {
            self.quote = None;
            self.last = Some(quote);
            self.i += 2;
        }
    }

    /// One step in code, after skipping the bytes that need no look; `false` when the text ends.
    #[inline]
    fn in_code(&mut self) -> bool {
        let text = self.text;
        while self.i < self.to && !PY_INTERESTING[text[self.i] as usize] {
            if !text[self.i].is_ascii_whitespace() {
                self.last = Some(text[self.i]);
            }
            self.i += 1;
        }
        if self.i >= self.to {
            return false;
        }
        let (i, byte) = (self.i, text[self.i]);
        match byte {
            b'#' => {
                // Stop just before the newline, which the next round handles.
                self.i = memchr::memchr(b'\n', &text[i..self.to]).map_or(self.to, |o| i + o) - 1;
            }
            b'\'' | b'"' => {
                let triple = text[i..].starts_with(&[byte, byte, byte]);
                self.quote = Some((byte, triple));
                if triple {
                    self.i += 2;
                }
            }
            b'(' | b'[' | b'{' => {
                self.depth += 1;
                self.last = Some(byte);
            }
            b')' | b']' | b'}' => {
                self.suspect |= self.depth == 0;
                self.depth = self.depth.saturating_sub(1);
                self.last = Some(byte);
            }
            b'\\' if text.get(i + 1) == Some(&b'\n') => self.continued = true,
            b'\\' if text[i + 1..].starts_with(b"\r\n") => self.continued = true,
            b'\\' => self.last = Some(byte),
            _ => {}
        }
        true
    }
}

/// Narrows each top-level `class`/`def` among `picked` to its header plus the body statements
/// holding a position, so a hit in one method does not parse the whole class. A position in the
/// header (decorators, signature, bases) keeps the whole statement: the match may be the class.
pub(super) fn python_members(text: &[u8], picked: &[Chunk], positions: &[usize]) -> Vec<Chunk> {
    let mut out = Vec::new();
    for &chunk in picked {
        let split = match chunk.body_indent {
            Some(level) => {
                python_split(text, chunk.start, chunk.end, chunk.start_row, level).chunks
            }
            None => Vec::new(),
        };
        match split.split_first() {
            Some((&header, members)) => narrow(chunk, header, members, None, positions, &mut out),
            None => out.push(chunk),
        }
    }
    merge(out)
}

/// Python's INDENT/DEDENT rules for one logical line: a block opened by `:` must indent, nothing
/// else may, and a dedent must land on an enclosing level.
fn indentation_ok(indents: &mut Vec<usize>, indent: usize, opens_block: bool) -> bool {
    let top = *indents.last().unwrap_or(&0);
    if opens_block {
        indents.push(indent);
        return indent > top;
    }
    if indent > top {
        return false;
    }
    while indents.last().is_some_and(|&level| level > indent) {
        indents.pop();
    }
    indents.last() == Some(&indent)
}

fn keyword_at(rest: &[u8], keyword: &[u8]) -> bool {
    rest.starts_with(keyword)
        && rest
            .get(keyword.len())
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
}
