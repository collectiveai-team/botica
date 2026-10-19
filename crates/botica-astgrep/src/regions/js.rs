use super::{build, merge, narrow, Chunk, Split};

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
pub(super) fn js_chunks(text: &[u8]) -> Split {
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
pub(super) fn js_members(text: &[u8], picked: &[Chunk], positions: &[usize]) -> Vec<Chunk> {
    let mut out = Vec::new();
    for &chunk in picked {
        match chunk.block.and_then(|b| js_body(text, chunk, b)) {
            Some((header, members, tail)) => {
                narrow(chunk, header, &members, Some(tail), positions, &mut out);
            }
            None => out.push(chunk),
        }
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
