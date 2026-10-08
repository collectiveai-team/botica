//! Experiment: parse only the top-level statements that hold a prefilter candidate.
//!
//! A rule's match lies inside one top-level statement (rules that look at siblings or at the
//! module itself are excluded by the caller), so the other statements need not be parsed.
//! tree-sitter's included ranges keep byte offsets global. Statement boundaries come from a
//! minimal lexer per language (Python, and JavaScript/TypeScript/TSX).

use ast_grep_language::SupportLang;

/// The top-level statements of `text`, or `None` for a language without a splitter.
pub fn chunks(language: SupportLang, text: &[u8]) -> Option<Split> {
    match language {
        SupportLang::Python => Some(python_chunks(text)),
        SupportLang::JavaScript | SupportLang::TypeScript | SupportLang::Tsx => {
            Some(js_chunks(text))
        }
        _ => None,
    }
}

/// A top-level statement: byte range and the row it starts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub start: usize,
    pub end: usize,
    pub start_row: usize,
    pub end_row: usize,
}

/// The top-level statements of a file, and whether the lexer saw a likely syntax error.
///
/// tree-sitter's error recovery in one statement can change how later statements parse, so a
/// file that may not parse must be parsed whole to match ast-grep. The lexer catches the common
/// cases (unbalanced brackets, unterminated strings, bad indentation), not every error.
pub struct Split {
    pub chunks: Vec<Chunk>,
    pub suspect: bool,
}

/// Splits Python source into top-level statements. Comment-only and blank lines at column 0 stay
/// with the statement before them; `else`/`elif`/`except`/`finally` and decorated definitions
/// stay with the statement they continue.
pub fn python_chunks(text: &[u8]) -> Split {
    let mut starts: Vec<(usize, usize)> = vec![(0, 0)];
    let mut depth = 0usize;
    let mut quote: Option<(u8, bool)> = None; // (quote byte, triple)
    let mut continued = false;
    let mut after_decorator = false;
    let mut indents: Vec<usize> = vec![0];
    let mut last: Option<u8> = None; // last significant code byte
    let mut suspect = false;
    let mut row = 0;
    let mut i = 0;
    let mut line_start = true;
    while i < text.len() {
        if line_start {
            line_start = false;
            if quote.is_none() && depth == 0 && !continued {
                let indent = text[i..]
                    .iter()
                    .take_while(|b| **b == b' ' || **b == b'\t')
                    .count();
                let first = text.get(i + indent).copied();
                if first.is_some_and(|b| b != b'\n' && b != b'\r' && b != b'#') {
                    suspect |= !indentation_ok(&mut indents, indent, last == Some(b':'));
                }
                let byte = text[i];
                if !byte.is_ascii_whitespace() && byte != b'#' {
                    let rest = &text[i..];
                    let continues = ["else", "elif", "except", "finally"]
                        .iter()
                        .any(|kw| keyword_at(rest, kw.as_bytes()));
                    if !continues && !after_decorator && i > 0 {
                        starts.push((i, row));
                    }
                    after_decorator = byte == b'@';
                }
            }
            continued = false;
        }
        let byte = text[i];
        match quote {
            Some((q, triple)) => {
                if byte == b'\\' {
                    i += 1;
                } else if byte == q {
                    if !triple {
                        quote = None;
                        last = Some(q);
                    } else if text[i..].starts_with(&[q, q, q]) {
                        quote = None;
                        last = Some(q);
                        i += 2;
                    }
                } else if byte == b'\n' && !triple {
                    quote = None;
                    suspect = true;
                }
            }
            None => match byte {
                b'#' => {
                    while i + 1 < text.len() && text[i + 1] != b'\n' {
                        i += 1;
                    }
                }
                b'\'' | b'"' => {
                    let triple = text[i..].starts_with(&[byte, byte, byte]);
                    quote = Some((byte, triple));
                    if triple {
                        i += 2;
                    }
                }
                b'(' | b'[' | b'{' => {
                    depth += 1;
                    last = Some(byte);
                }
                b')' | b']' | b'}' => {
                    suspect |= depth == 0;
                    depth = depth.saturating_sub(1);
                    last = Some(byte);
                }
                b'\\' if text.get(i + 1) == Some(&b'\n') => continued = true,
                b'\\' if text[i + 1..].starts_with(b"\r\n") => continued = true,
                _ if !byte.is_ascii_whitespace() => last = Some(byte),
                _ => {}
            },
        }
        if text.get(i) == Some(&b'\n') {
            row += 1;
            line_start = true;
        }
        i += 1;
    }
    Split {
        chunks: build(&starts, text.len(), row),
        suspect: suspect || depth != 0 || quote.is_some(),
    }
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

fn build(starts: &[(usize, usize)], text_len: usize, total_rows: usize) -> Vec<Chunk> {
    let mut chunks = Vec::with_capacity(starts.len());
    for (k, &(start, start_row)) in starts.iter().enumerate() {
        let (end, end_row) = starts.get(k + 1).copied().unwrap_or((text_len, total_rows));
        chunks.push(Chunk {
            start,
            end,
            start_row,
            end_row,
        });
    }
    chunks
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Js {
    Code,
    Str(u8),
    Template,
    Line,
    Block,
    Regex { class: bool },
}

/// Splits JavaScript/TypeScript/TSX source into top-level statements. Strings end at a newline
/// and regex literals too, so JSX text with a stray quote only costs splits or a `suspect` flag,
/// not correctness: a wrong split leaves a partial tree with errors, and the caller then parses
/// the whole file. Unterminated strings are not flagged, since JSX text holds stray quotes.
pub fn js_chunks(text: &[u8]) -> Split {
    let mut suspect = false;
    let mut starts: Vec<(usize, usize)> = vec![(0, 0)];
    let mut mode = if text.starts_with(b"#!") {
        Js::Line
    } else {
        Js::Code
    };
    let mut depth = 0usize;
    let mut templates: Vec<usize> = Vec::new();
    let mut last: Option<usize> = None;
    let mut after_decorator = false;
    let mut row = 0;
    let mut i = 0;
    let mut line_start = true;
    while i < text.len() {
        if line_start {
            line_start = false;
            if mode == Js::Code && depth == 0 && templates.is_empty() {
                if let Some(word) = js_statement_word(&text[i..]) {
                    let ends = last.is_none_or(|p| !b"=+-*/%&|^!?:,.(<>[{~".contains(&text[p]));
                    if ends && !after_decorator && i > 0 && JS_STARTS.contains(&word) {
                        starts.push((i, row));
                    }
                    after_decorator = word == "@";
                }
            }
        }
        let byte = text[i];
        match mode {
            Js::Str(quote) => {
                if byte == b'\\' {
                    i += 1;
                } else if byte == quote || byte == b'\n' {
                    mode = Js::Code;
                    last = Some(i);
                }
            }
            Js::Template => {
                if byte == b'\\' {
                    i += 1;
                } else if byte == b'`' {
                    mode = Js::Code;
                    last = Some(i);
                } else if byte == b'$' && text.get(i + 1) == Some(&b'{') {
                    templates.push(depth);
                    depth += 1;
                    mode = Js::Code;
                    i += 1;
                    last = Some(i);
                }
            }
            Js::Line => {
                if byte == b'\n' {
                    mode = Js::Code;
                }
            }
            Js::Block => {
                if text[i..].starts_with(b"*/") {
                    mode = Js::Code;
                    i += 1;
                }
            }
            Js::Regex { class } => {
                if byte == b'\\' {
                    i += 1;
                } else if byte == b'\n' {
                    mode = Js::Code;
                } else if class {
                    if byte == b']' {
                        mode = Js::Regex { class: false };
                    }
                } else if byte == b'[' {
                    mode = Js::Regex { class: true };
                } else if byte == b'/' {
                    mode = Js::Code;
                    last = Some(i);
                }
            }
            Js::Code => match byte {
                b'/' if text.get(i + 1) == Some(&b'/') => mode = Js::Line,
                b'/' if text.get(i + 1) == Some(&b'*') => {
                    mode = Js::Block;
                    i += 1;
                }
                b'/' if js_regex_allowed(text, last) => mode = Js::Regex { class: false },
                b'\'' | b'"' => mode = Js::Str(byte),
                b'`' => mode = Js::Template,
                b'(' | b'[' | b'{' => {
                    depth += 1;
                    last = Some(i);
                }
                b')' | b']' | b'}' => {
                    suspect |= depth == 0;
                    depth = depth.saturating_sub(1);
                    last = Some(i);
                    if byte == b'}' && templates.last() == Some(&depth) {
                        templates.pop();
                        mode = Js::Template;
                    }
                }
                _ if !byte.is_ascii_whitespace() => last = Some(i),
                _ => {}
            },
        }
        if text.get(i) == Some(&b'\n') {
            row += 1;
            line_start = true;
        }
        i += 1;
    }
    Split {
        chunks: build(&starts, text.len(), row),
        suspect: suspect
            || depth != 0
            || !templates.is_empty()
            || matches!(mode, Js::Template | Js::Block),
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

fn keyword_at(rest: &[u8], keyword: &[u8]) -> bool {
    rest.starts_with(keyword)
        && rest
            .get(keyword.len())
            .is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'_'))
}

/// The chunks holding at least one of `positions`, merged when adjacent.
pub fn select(chunks: &[Chunk], positions: &mut [usize]) -> Vec<Chunk> {
    positions.sort_unstable();
    let mut out: Vec<Chunk> = Vec::new();
    let mut k = 0;
    for &position in positions.iter() {
        while k < chunks.len() && chunks[k].end <= position {
            k += 1;
        }
        let Some(chunk) = chunks.get(k) else { break };
        match out.last_mut() {
            Some(last) if last.end == chunk.start => {
                last.end = chunk.end;
                last.end_row = chunk.end_row;
            }
            Some(last) if last.start == chunk.start => {}
            _ => out.push(*chunk),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(source: &str) -> Vec<&str> {
        python_chunks(source.as_bytes())
            .chunks
            .iter()
            .map(|c| source[c.start..c.end].lines().next().unwrap_or(""))
            .collect()
    }

    #[test]
    fn splits_at_column_zero_statements() {
        let source = "import os\n\ndef f():\n    return 1\n\nx = 2\n";
        assert_eq!(starts(source), ["import os", "def f():", "x = 2"]);
    }

    #[test]
    fn strings_brackets_and_continuations_do_not_split() {
        let source = "s = \"\"\"\nnot = a statement\n\"\"\"\ny = (\n1)\nz = 1 + \\\n2\nw = 'a#b'\n";
        assert_eq!(
            starts(source),
            ["s = \"\"\"", "y = (", "z = 1 + \\", "w = 'a#b'"]
        );
    }

    #[test]
    fn decorators_and_clause_keywords_continue_the_statement() {
        let source = "@dec\n@other\ndef f():\n    pass\ntry:\n    x\nexcept E:\n    y\nfinally:\n    z\n# note\nif a:\n    b\nelse:\n    c\n";
        assert_eq!(starts(source), ["@dec", "try:", "if a:"]);
    }

    fn js_starts(source: &str) -> Vec<&str> {
        js_chunks(source.as_bytes())
            .chunks
            .iter()
            .map(|c| source[c.start..c.end].lines().next().unwrap_or(""))
            .collect()
    }

    #[test]
    fn js_splits_before_statement_keywords_only() {
        let source = "import a from 'a'\nconst x =\nfoo()\nfunction f() {\nconst y = 1\n}\nfoo()\nexport default f\n";
        assert_eq!(
            js_starts(source),
            [
                "import a from 'a'",
                "const x =",
                "function f() {",
                "export default f"
            ]
        );
    }

    #[test]
    fn js_strings_templates_regexes_and_decorators_do_not_split() {
        let source = concat!(
            "const t = `a ${ {b: `\nconst inner = 1` } } c\nexport x`\n",
            "const r = /[/]const/g\n",
            "// const in a comment\n/* \nconst in a block */\n",
            "@Component({\n})\nexport class C {}\n",
            "let d = a / b\nlet e = 1\n",
        );
        assert_eq!(
            js_starts(source),
            [
                "const t = `a ${ {b: `",
                "const r = /[/]const/g",
                "@Component({",
                "let d = a / b",
                "let e = 1"
            ]
        );
    }

    #[test]
    fn likely_syntax_errors_are_flagged() {
        let python = |s: &str| python_chunks(s.as_bytes()).suspect;
        assert!(!python("class A:\n    def f(self):\n        if x:  # c\n            pass\n    y = 1\nz = (\n  1)\n"));
        assert!(python("x = 1\n    y = 2\n")); // unexpected indent
        assert!(python("def f():\nreturn 1\n")); // expected an indented block
        assert!(python("if a:\n        b\n    c\n")); // dedent to no enclosing level
        assert!(python("f(a))\n")); // unmatched ')'
        assert!(python("x = (1,\n")); // never closed
        assert!(python("s = 'abc\n")); // unterminated string
        assert!(python("s = \"\"\"abc\n")); // unterminated triple quote

        let js = |s: &str| js_chunks(s.as_bytes()).suspect;
        assert!(!js(
            "const a = `x ${b} y`\nfunction f() { return /}/ }\nconst p = <p>Don't</p>\n"
        ));
        assert!(js("f(a))\n"));
        assert!(js("function f() {\n"));
        assert!(js("const t = `abc\n"));
        assert!(js("/* open\n"));
    }

    #[test]
    fn select_merges_adjacent_chunks() {
        let chunks = python_chunks(b"a = 1\nb = 2\nc = 3\nd = 4\n").chunks;
        let picked = select(&chunks, &mut [7, 2, 20, 21]);
        assert_eq!(
            picked
                .iter()
                .map(|c| (c.start, c.end, c.start_row, c.end_row))
                .collect::<Vec<_>>(),
            [(0, 12, 0, 2), (18, 24, 3, 4)]
        );
    }
}
