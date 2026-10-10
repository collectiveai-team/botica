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
