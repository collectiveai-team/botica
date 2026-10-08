//! Region parsing: parse only the top-level statements that hold a prefilter candidate.
//!
//! A rule's match lies inside one top-level statement (rules that look at siblings or at the
//! module itself are excluded by the caller), so the other statements need not be parsed.
//! tree-sitter's included ranges keep byte offsets global. Statement boundaries come from a
//! minimal lexer per language (Python, and JavaScript/TypeScript/TSX). Inside a top-level class
//! or function, `members` narrows further to the header and the members that hold a candidate.

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

/// A statement: byte range and the rows it spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub start: usize,
    pub end: usize,
    pub start_row: usize,
    pub end_row: usize,
    /// For a Python `class`/`def`: the indentation of its body, for `python_members`.
    pub body_indent: Option<usize>,
    /// For a JS/TS statement ending in a `{…}` (a class, function or arrow body, a block or an
    /// object): its `{` and `}`, for `js_members`.
    pub block: Option<(usize, usize)>,
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
    // (start, row, body indent if the statement is a class/def)
    let mut starts: Vec<(usize, usize, Option<usize>)> = vec![(from, from_row, None)];
    let mut head_seen = false;
    let mut compound = false;
    let mut depth = 0usize;
    let mut quote: Option<(u8, bool)> = None; // (quote byte, triple)
    let mut continued = false;
    let mut after_decorator = false;
    let mut indents: Vec<usize> = vec![0];
    let mut last: Option<u8> = None; // last significant code byte
    let mut suspect = false;
    let mut row = from_row;
    let mut i = from;
    let mut line_start = true;
    while i < to {
        if line_start {
            line_start = false;
            if quote.is_none() && depth == 0 && !continued {
                let indent = text[i..to]
                    .iter()
                    .take_while(|b| **b == b' ' || **b == b'\t')
                    .count();
                let first = text.get(i + indent).copied().filter(|_| i + indent < to);
                if first.is_some_and(|b| b != b'\n' && b != b'\r' && b != b'#') {
                    if level == 0 {
                        suspect |= !indentation_ok(&mut indents, indent, last == Some(b':'));
                    }
                    let current = starts.len() - 1;
                    if indent > level && compound && starts[current].2.is_none() {
                        starts[current].2 = Some(indent);
                    }
                    if indent == level {
                        let rest = &text[i + indent..to];
                        let continues = ["else", "elif", "except", "finally"]
                            .iter()
                            .any(|kw| keyword_at(rest, kw.as_bytes()));
                        if !continues && !after_decorator && i > from {
                            starts.push((i, row, None));
                            head_seen = false;
                            compound = false;
                        }
                        after_decorator = rest[0] == b'@';
                        if !after_decorator && !head_seen {
                            head_seen = true;
                            compound = keyword_at(rest, b"class")
                                || keyword_at(rest, b"def")
                                || keyword_at(rest, b"async");
                        }
                    }
                }
            }
            continued = false;
        }
        match quote {
            Some((q, triple)) => {
                match memchr::memchr3(q, b'\\', b'\n', &text[i..to]) {
                    Some(offset) => i += offset,
                    None => break,
                }
                let byte = text[i];
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
                } else if !triple {
                    quote = None;
                    suspect = true;
                }
            }
            None => {
                while i < to && !PY_INTERESTING[text[i] as usize] {
                    if !text[i].is_ascii_whitespace() {
                        last = Some(text[i]);
                    }
                    i += 1;
                }
                if i >= to {
                    break;
                }
                let byte = text[i];
                match byte {
                    b'#' => {
                        // Stop just before the newline, which the next round handles.
                        i = memchr::memchr(b'\n', &text[i..to]).map_or(to, |o| i + o) - 1;
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
                    b'\\' => last = Some(byte),
                    _ => {}
                }
            }
        }
        if text.get(i) == Some(&b'\n') {
            row += 1;
            line_start = true;
        }
        i += 1;
    }
    Split {
        chunks: build(&starts, to, row),
        suspect: suspect || depth != 0 || quote.is_some(),
    }
}

/// Narrows each top-level `class`/`def` among `picked` to its header plus the body statements
/// holding a position, so a hit in one method does not parse the whole class. A position in the
/// header (decorators, signature, bases) keeps the whole statement: the match may be the class.
pub fn python_members(text: &[u8], picked: &[Chunk], positions: &[usize]) -> Vec<Chunk> {
    let mut out = Vec::new();
    for &chunk in picked {
        let members = match chunk.body_indent {
            Some(level) => {
                python_split(text, chunk.start, chunk.end, chunk.start_row, level).chunks
            }
            None => Vec::new(),
        };
        let mut inside: Vec<usize> = positions
            .iter()
            .copied()
            .filter(|&p| p >= chunk.start && p < chunk.end)
            .collect();
        if members.len() < 2 || inside.iter().any(|&p| p < members[1].start) {
            out.push(chunk);
            continue;
        }
        out.push(members[0]);
        out.extend(pick(&members[1..], &mut inside));
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

fn build(
    starts: &[(usize, usize, Option<usize>)],
    text_len: usize,
    total_rows: usize,
) -> Vec<Chunk> {
    let mut chunks = Vec::with_capacity(starts.len());
    for (k, &(start, start_row, body_indent)) in starts.iter().enumerate() {
        let (end, end_row) = starts
            .get(k + 1)
            .map_or((text_len, total_rows), |&(s, r, _)| (s, r));
        chunks.push(Chunk {
            start,
            end,
            start_row,
            end_row,
            body_indent,
            block: None,
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
    js_split(text, 0, text.len(), 0, false)
}

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

/// Splits `text[from..to]` into statements. At the top level (`body == false`) it splits before
/// the words of `JS_STARTS` and after the body of a `class`/`function` declaration, and records
/// the block each statement ends in. Inside a body it
/// splits before a line that starts with a word, `@` or `#` right after a `;` or `}`: members
/// without semicolons stay with the one before, which only narrows less.
fn js_split(text: &[u8], from: usize, to: usize, from_row: usize, body: bool) -> Split {
    let mut suspect = false;
    let mut starts: Vec<(usize, usize, Option<usize>)> = vec![(from, from_row, None)];
    let mut blocks: Vec<Option<(usize, usize)>> = Vec::new();
    let mut mode = if from == 0 && text.starts_with(b"#!") {
        Js::Line
    } else {
        Js::Code
    };
    let mut depth = 0usize;
    let mut templates: Vec<usize> = Vec::new();
    let mut last: Option<usize> = None;
    let mut after_decorator = false;
    let mut head_seen = false;
    let mut declaration = false;
    let mut open = 0usize;
    let mut block: Option<(usize, usize)> = None;
    let mut row = from_row;
    let mut i = from;
    let mut line_start = true;
    while i < to {
        if line_start {
            line_start = false;
            if mode == Js::Code && depth == 0 && templates.is_empty() {
                if body {
                    let indent = text[i..to]
                        .iter()
                        .take_while(|b| **b == b' ' || **b == b'\t')
                        .count();
                    let rest = &text[i + indent..to];
                    let word = match rest.first() {
                        Some(b'#') => Some("#"),
                        _ => js_statement_word(rest),
                    };
                    let ended = last.is_some_and(|p| text[p] == b';' || text[p] == b'}');
                    if ended && i > from && word.is_some_and(|w| !JS_CONTINUES.contains(&w)) {
                        starts.push((i, row, None));
                    }
                } else {
                    let rest = &text[i..to];
                    let word = js_statement_word(rest);
                    let code = rest.first().is_some_and(|b| !b.is_ascii_whitespace())
                        && !rest.starts_with(b"//")
                        && !rest.starts_with(b"/*");
                    let ends = last.is_none_or(|p| !b"=+-*/%&|^!?:,.(<>[{~".contains(&text[p]));
                    let starts_statement = ends && word.is_some_and(|w| JS_STARTS.contains(&w));
                    // A class or function declaration ends at its body.
                    let after_body = declaration && ending_block(text, block, last).is_some();
                    if (starts_statement || (after_body && code)) && !after_decorator && i > from {
                        blocks.push(ending_block(text, block, last));
                        starts.push((i, row, None));
                        head_seen = false;
                        declaration = false;
                        block = None;
                    }
                    if code {
                        after_decorator = word == Some("@");
                        if !after_decorator && !head_seen {
                            head_seen = true;
                            declaration = js_declaration(rest);
                        }
                    }
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
                    if byte == b'{' && depth == 0 {
                        open = i;
                    }
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
                    } else if byte == b'}' && depth == 0 {
                        block = Some((open, i));
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
    let mut chunks = build(&starts, to, row);
    if !body {
        blocks.push(ending_block(text, block, last));
        for (chunk, block) in chunks.iter_mut().zip(blocks) {
            chunk.block = block;
        }
    }
    Split {
        chunks,
        suspect: suspect
            || depth != 0
            || !templates.is_empty()
            || matches!(mode, Js::Template | Js::Block),
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

/// Narrows each statement among `picked` that ends in a block (see `Chunk::block`) to its header,
/// the body members holding a position, and the closing `}`. A position in the header or after
/// the body keeps the whole statement: the match may be the definition itself.
pub fn js_members(text: &[u8], picked: &[Chunk], positions: &[usize]) -> Vec<Chunk> {
    let mut out = Vec::new();
    for &chunk in picked {
        let Some((header, members, tail)) = chunk.block.and_then(|b| js_body(text, chunk, b))
        else {
            out.push(chunk);
            continue;
        };
        let mut inside: Vec<usize> = positions
            .iter()
            .copied()
            .filter(|&p| p >= chunk.start && p < chunk.end)
            .collect();
        if members.len() < 2 || inside.iter().any(|&p| p < header.end || p >= tail.start) {
            out.push(chunk);
            continue;
        }
        out.push(header);
        out.extend(pick(&members, &mut inside));
        out.push(tail);
    }
    merge(out)
}

/// Splits a definition into header (through the line of `{`), body members, and the line of the
/// closing `}` to the end of the statement. `None` unless `{` ends its line and `}` starts one.
fn js_body(
    text: &[u8],
    chunk: Chunk,
    (open, close): (usize, usize),
) -> Option<(Chunk, Vec<Chunk>, Chunk)> {
    let after = memchr::memchr(b'\n', &text[open + 1..close])?;
    if !text[open + 1..open + 1 + after]
        .iter()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r'))
    {
        return None;
    }
    let body_start = open + after + 2;
    let close_line =
        memchr::memrchr(b'\n', &text[body_start..close]).map(|p| body_start + p + 1)?;
    if !text[close_line..close]
        .iter()
        .all(|b| *b == b' ' || *b == b'\t')
    {
        return None;
    }
    let body_row =
        chunk.start_row + memchr::memchr_iter(b'\n', &text[chunk.start..body_start]).count();
    let members = js_split(text, body_start, close_line, body_row, true).chunks;
    let close_row = members.last().map_or(body_row, |m| m.end_row);
    let header = Chunk {
        end: body_start,
        end_row: body_row,
        block: None,
        ..chunk
    };
    let tail = Chunk {
        start: close_line,
        start_row: close_row,
        block: None,
        ..chunk
    };
    Some((header, members, tail))
}

/// Narrows the picked top-level statements to class/function members, for languages that can.
pub fn members(
    language: SupportLang,
    text: &[u8],
    picked: &[Chunk],
    positions: &[usize],
) -> Vec<Chunk> {
    match language {
        SupportLang::Python => python_members(text, picked, positions),
        SupportLang::JavaScript | SupportLang::TypeScript | SupportLang::Tsx => {
            js_members(text, picked, positions)
        }
        _ => merge(picked.to_vec()),
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
#[cfg(test)]
fn select(chunks: &[Chunk], positions: &mut [usize]) -> Vec<Chunk> {
    merge(pick(chunks, positions))
}

/// The chunks holding at least one of `positions`, each once, in order.
pub fn pick(chunks: &[Chunk], positions: &mut [usize]) -> Vec<Chunk> {
    positions.sort_unstable();
    let mut out: Vec<Chunk> = Vec::new();
    let mut k = 0;
    for &position in positions.iter() {
        while k < chunks.len() && chunks[k].end <= position {
            k += 1;
        }
        let Some(chunk) = chunks.get(k) else { break };
        if out.last().is_none_or(|last| last.start != chunk.start) {
            out.push(*chunk);
        }
    }
    out
}

/// Joins chunks that touch, so the parser gets fewer ranges.
pub fn merge(chunks: Vec<Chunk>) -> Vec<Chunk> {
    let mut out: Vec<Chunk> = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        match out.last_mut() {
            Some(last) if last.end == chunk.start => {
                last.end = chunk.end;
                last.end_row = chunk.end_row;
            }
            _ => out.push(chunk),
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
                "foo()",
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

    fn narrowed(source: &str, needle: &str) -> String {
        let text = source.as_bytes();
        let mut positions: Vec<usize> = source.match_indices(needle).map(|(i, _)| i).collect();
        let top = pick(&python_chunks(text).chunks, &mut positions);
        python_members(text, &top, &positions)
            .iter()
            .map(|c| &source[c.start..c.end])
            .collect()
    }

    #[test]
    fn classes_and_functions_narrow_to_header_and_members_with_candidates() {
        let source = "import os\n\n@dec(\n    1,\n)\nclass A(\n    Base,\n):\n    '''doc'''\n\n    def f(self):\n        return 1\n\n    @prop\n    def g(self):\n        print(2)\n\n    x = 3\n\n\ndef h():\n    a = 1\n    print(3)\n";
        assert_eq!(
            narrowed(source, "print"),
            "@dec(\n    1,\n)\nclass A(\n    Base,\n):\n    @prop\n    def g(self):\n        print(2)\n\n    \
             def h():\n    print(3)\n"
                .replace("    def h", "def h")
        );
    }

    #[test]
    fn a_candidate_in_the_header_keeps_the_whole_class() {
        let source = "class A(Base):\n    def f(self):\n        return 1\n    def g(self):\n        return 2\n";
        assert_eq!(narrowed(source, "Base"), source);
    }

    fn js_narrowed(source: &str, needle: &str) -> String {
        let text = source.as_bytes();
        let mut positions: Vec<usize> = source.match_indices(needle).map(|(i, _)| i).collect();
        let top = pick(&js_chunks(text).chunks, &mut positions);
        js_members(text, &top, &positions)
            .iter()
            .map(|c| &source[c.start..c.end])
            .collect()
    }

    #[test]
    fn js_classes_and_functions_narrow_to_header_members_and_closing_brace() {
        let source = concat!(
            "import a from 'a'\n",
            "@Dec()\nexport class A extends B<{ x: number }> {\n",
            "  f() {\n    return 1\n  }\n\n",
            "  @Input()\n  g = console.log(2);\n\n",
            "  h() {\n    console.log(3)\n  }\n",
            "}\n",
            "function k() {\n  const a = 1;\n  if (a) {\n  }\n  else {\n    console.log(4);\n  }\n}\n",
        );
        assert_eq!(
            js_narrowed(source, "console"),
            concat!(
                "@Dec()\nexport class A extends B<{ x: number }> {\n",
                "  @Input()\n  g = console.log(2);\n\n",
                "  h() {\n    console.log(3)\n  }\n",
                "}\n",
                "function k() {\n  if (a) {\n  }\n  else {\n    console.log(4);\n  }\n}\n",
            )
        );
    }

    #[test]
    fn js_keeps_whole_statements_it_cannot_narrow() {
        // A candidate in the header, an object literal, and members without semicolons.
        let header = "class A extends Base {\n  f() {}\n  g() {}\n}\n";
        assert_eq!(js_narrowed(header, "Base"), header);
        let object = "export const o = {\n  a: f(1),\n  b: 2,\n}\n";
        assert_eq!(js_narrowed(object, "f(1)"), object);
        let loose = "class A {\n  x = 1\n  y = f()\n}\n";
        assert_eq!(js_narrowed(loose, "f()"), loose);
    }

    #[test]
    fn js_declarations_end_at_their_body_and_expressions_narrow_too() {
        let source = concat!(
            "class A {\n  f() {}\n  g() { log() }\n}\nexports.A = A;\n",
            "module.exports = cls => class B extends cls {\n  h() {}\n  i() { log() }\n};\n",
            "const k = () => {\n  a();\n  log();\n}\n",
        );
        assert_eq!(
            js_narrowed(source, "log"),
            concat!(
                "class A {\n  g() { log() }\n}\n",
                "module.exports = cls => class B extends cls {\n  i() { log() }\n};\n",
                "const k = () => {\n  log();\n}\n",
            )
        );
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
