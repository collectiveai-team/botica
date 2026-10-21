use super::super::{build, Split};

/// Words that open a top-level JS/TS statement. Splitting only before these (at column 0, outside
/// any bracket, after a line that does not end in an operator) keeps ASI continuations together.
const JS_STARTS: [&str; 16] = [
    "import",
    "export",
    "const",
    "let",
    "var",
    "function",
    "class",
    "interface",
    "type",
    "declare",
    "enum",
    "namespace",
    "module",
    "abstract",
    "async",
    "@",
];

/// After these words a `/` starts a regex literal, not a division.
const JS_REGEX_AFTER: [&str; 14] = [
    "return",
    "typeof",
    "case",
    "do",
    "else",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "throw",
    "instanceof",
    "yield",
    "await",
];

/// After a member ending in `;` or `}`, a line starting with one of these continues it.
const JS_CONTINUES: [&str; 9] = [
    "else",
    "catch",
    "finally",
    "while",
    "in",
    "instanceof",
    "as",
    "satisfies",
    "of",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Js {
    Code,
    Str(u8),
    Template,
    Line,
    Block,
    Regex { class: bool },
}

/// Splits `text[from..to]` into statements. At the top level (`body == false`) it splits before
/// the words of `JS_STARTS` and after the body of a `class`/`function` declaration, and records
/// the block each statement ends in. Inside a body it
/// splits before a line that starts with a word, `@` or `#` right after a `;` or `}`: members
/// without semicolons stay with the one before, which only narrows less.
pub(super) fn js_split(text: &[u8], from: usize, to: usize, from_row: usize, body: bool) -> Split {
    let mut lexer = JsLexer::new(text, from, to, from_row, body);
    lexer.run();
    lexer.finish()
}

/// The state of `js_split` while it walks `text[from..to]`.
struct JsLexer<'a> {
    text: &'a [u8],
    from: usize,
    to: usize,
    body: bool,
    suspect: bool,
    starts: Vec<(usize, usize, Option<usize>)>,
    blocks: Vec<Option<(usize, usize)>>,
    mode: Js,
    depth: usize,
    templates: Vec<usize>,
    last: Option<usize>,
    after_decorator: bool,
    head_seen: bool,
    declaration: bool,
    open: usize,
    block: Option<(usize, usize)>,
    row: usize,
    i: usize,
}

impl<'a> JsLexer<'a> {
    fn new(text: &'a [u8], from: usize, to: usize, from_row: usize, body: bool) -> Self {
        let mode = if from == 0 && text.starts_with(b"#!") {
            Js::Line
        } else {
            Js::Code
        };
        Self {
            text,
            from,
            to,
            body,
            suspect: false,
            starts: vec![(from, from_row, None)],
            blocks: Vec::new(),
            mode,
            depth: 0,
            templates: Vec::new(),
            last: None,
            after_decorator: false,
            head_seen: false,
            declaration: false,
            open: 0,
            block: None,
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
            self.step(self.text[self.i]);
            line_start = self.text.get(self.i) == Some(&b'\n');
            if line_start {
                self.row += 1;
            }
            self.i += 1;
        }
    }

    fn finish(mut self) -> Split {
        let mut chunks = build(&self.starts, self.to, self.row);
        if !self.body {
            self.blocks
                .push(ending_block(self.text, self.block, self.last));
            for (chunk, block) in chunks.iter_mut().zip(self.blocks) {
                chunk.block = block;
            }
        }
        Split {
            chunks,
            suspect: self.suspect
                || self.depth != 0
                || !self.templates.is_empty()
                || matches!(self.mode, Js::Template | Js::Block),
        }
    }

    #[inline]
    fn at_line_start(&mut self) {
        if self.mode != Js::Code || self.depth != 0 || !self.templates.is_empty() {
            return;
        }
        if self.body {
            self.member_line();
        } else {
            self.statement_line();
        }
    }

    /// A line inside a body: starts a member after a `;` or `}`.
    fn member_line(&mut self) {
        let (text, i, to) = (self.text, self.i, self.to);
        let indent = text[i..to]
            .iter()
            .take_while(|b| **b == b' ' || **b == b'\t')
            .count();
        let rest = &text[i + indent..to];
        let word = match rest.first() {
            Some(b'#') => Some("#"),
            _ => js_statement_word(rest),
        };
        let ended = self
            .last
            .is_some_and(|p| text[p] == b';' || text[p] == b'}');
        if ended && i > self.from && word.is_some_and(|w| !JS_CONTINUES.contains(&w)) {
            self.starts.push((i, self.row, None));
        }
    }

    /// A line at the top level: starts a statement before a word of `JS_STARTS` or after the
    /// body of a declaration.
    fn statement_line(&mut self) {
        let (text, i, to) = (self.text, self.i, self.to);
        let rest = &text[i..to];
        let word = js_statement_word(rest);
        let code = rest.first().is_some_and(|b| !b.is_ascii_whitespace())
            && !rest.starts_with(b"//")
            && !rest.starts_with(b"/*");
        let ends = self
            .last
            .is_none_or(|p| !b"=+-*/%&|^!?:,.(<>[{~".contains(&text[p]));
        let starts_statement = ends && word.is_some_and(|w| JS_STARTS.contains(&w));
        // A class or function declaration ends at its body.
        let after_body = self.declaration && ending_block(text, self.block, self.last).is_some();
        if (starts_statement || (after_body && code)) && !self.after_decorator && i > self.from {
            self.start_statement();
        }
        if code {
            self.after_decorator = word == Some("@");
            if !self.after_decorator && !self.head_seen {
                self.head_seen = true;
                self.declaration = js_declaration(rest);
            }
        }
    }

    fn start_statement(&mut self) {
        self.blocks
            .push(ending_block(self.text, self.block, self.last));
        self.starts.push((self.i, self.row, None));
        self.head_seen = false;
        self.declaration = false;
        self.block = None;
    }

    #[inline]
    fn step(&mut self, byte: u8) {
        match self.mode {
            Js::Str(quote) => self.in_string(byte, quote),
            Js::Template => self.in_template(byte),
            Js::Line => {
                if byte == b'\n' {
                    self.mode = Js::Code;
                }
            }
            Js::Block => {
                if self.text[self.i..].starts_with(b"*/") {
                    self.mode = Js::Code;
                    self.i += 1;
                }
            }
            Js::Regex { class } => self.in_regex(byte, class),
            Js::Code => self.in_code(byte),
        }
    }

    #[inline]
    fn in_string(&mut self, byte: u8, quote: u8) {
        if byte == b'\\' {
            self.i += 1;
        } else if byte == quote || byte == b'\n' {
            self.mode = Js::Code;
            self.last = Some(self.i);
        }
    }

    #[inline]
    fn in_template(&mut self, byte: u8) {
        if byte == b'\\' {
            self.i += 1;
        } else if byte == b'`' {
            self.mode = Js::Code;
            self.last = Some(self.i);
        } else if byte == b'$' && self.text.get(self.i + 1) == Some(&b'{') {
            self.templates.push(self.depth);
            self.depth += 1;
            self.mode = Js::Code;
            self.i += 1;
            self.last = Some(self.i);
        }
    }

    #[inline]
    fn in_regex(&mut self, byte: u8, class: bool) {
        if byte == b'\\' {
            self.i += 1;
        } else if byte == b'\n' {
            self.mode = Js::Code;
        } else if class {
            if byte == b']' {
                self.mode = Js::Regex { class: false };
            }
        } else if byte == b'[' {
            self.mode = Js::Regex { class: true };
        } else if byte == b'/' {
            self.mode = Js::Code;
            self.last = Some(self.i);
        }
    }

    #[inline]
    fn in_code(&mut self, byte: u8) {
        let (text, i) = (self.text, self.i);
        match byte {
            b'/' if text.get(i + 1) == Some(&b'/') => self.mode = Js::Line,
            b'/' if text.get(i + 1) == Some(&b'*') => {
                self.mode = Js::Block;
                self.i += 1;
            }
            b'/' if js_regex_allowed(text, self.last) => self.mode = Js::Regex { class: false },
            b'\'' | b'"' => self.mode = Js::Str(byte),
            b'`' => self.mode = Js::Template,
            b'(' | b'[' | b'{' => {
                if byte == b'{' && self.depth == 0 {
                    self.open = i;
                }
                self.depth += 1;
                self.last = Some(i);
            }
            b')' | b']' | b'}' => self.close_bracket(byte),
            _ if !byte.is_ascii_whitespace() => self.last = Some(i),
            _ => {}
        }
    }

    #[inline]
    fn close_bracket(&mut self, byte: u8) {
        self.suspect |= self.depth == 0;
        self.depth = self.depth.saturating_sub(1);
        self.last = Some(self.i);
        if byte == b'}' && self.templates.last() == Some(&self.depth) {
            self.templates.pop();
            self.mode = Js::Template;
        } else if byte == b'}' && self.depth == 0 {
            self.block = Some((self.open, self.i));
        }
    }
}

/// The statement's last top-level `{…}`, if nothing but a `;` follows it: the body of a class,
/// function or arrow, a block, or an object literal. Taking the last one skips braces in a
/// header, such as `extends B<{ x: number }>`.
fn ending_block(
    text: &[u8],
    block: Option<(usize, usize)>,
    last: Option<usize>,
) -> Option<(usize, usize)> {
    let (open, close) = block?;
    let last = last?;
    let ends_there = last == close
        || (text[last] == b';' && text[close + 1..last].iter().all(u8::is_ascii_whitespace));
    ends_there.then_some((open, close))
}

/// Whether a statement starts a `class` or `function` declaration, after its modifiers.
fn js_declaration(mut rest: &[u8]) -> bool {
    loop {
        let Some(word) = js_statement_word(rest) else {
            return false;
        };
        match word {
            "export" | "default" | "declare" | "abstract" | "async" => {
                rest = &rest[word.len()..];
                let blank = rest
                    .iter()
                    .take_while(|b| **b == b' ' || **b == b'\t')
                    .count();
                if blank == 0 {
                    return false;
                }
                rest = &rest[blank..];
            }
            "class" | "function" => return true,
            _ => return false,
        }
    }
}

fn is_js_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= 0x80
}

/// The identifier (or `@`) a line starts with, if it starts with one.
fn js_statement_word(rest: &[u8]) -> Option<&str> {
    match rest.first()? {
        b'@' => Some("@"),
        b if is_js_word(*b) && !b.is_ascii_digit() => {
            let len = rest.iter().take_while(|b| is_js_word(**b)).count();
            std::str::from_utf8(&rest[..len]).ok()
        }
        _ => None,
    }
}

fn js_regex_allowed(text: &[u8], last: Option<usize>) -> bool {
    let Some(end) = last else { return true };
    let byte = text[end];
    if is_js_word(byte) {
        let start = text[..=end]
            .iter()
            .rposition(|b| !is_js_word(*b))
            .map_or(0, |p| p + 1);
        return std::str::from_utf8(&text[start..=end])
            .is_ok_and(|word| JS_REGEX_AFTER.contains(&word));
    }
    b"(,=:[!&|?{};+-*%<>~^".contains(&byte)
}
