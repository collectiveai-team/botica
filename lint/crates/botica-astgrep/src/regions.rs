//! Experiment: parse only the top-level statements that hold a prefilter candidate.
//!
//! A rule's match lies inside one top-level statement (rules that look at siblings or at the
//! module itself are excluded by the caller), so the other statements need not be parsed.
//! tree-sitter's included ranges keep byte offsets global. Python only: statement boundaries
//! come from a minimal lexer (strings, brackets, line continuations, decorators).

/// A top-level statement: byte range and the row it starts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub start: usize,
    pub end: usize,
    pub start_row: usize,
    pub end_row: usize,
}

/// Splits Python source into top-level statements. Comment-only and blank lines at column 0 stay
/// with the statement before them; `else`/`elif`/`except`/`finally` and decorated definitions
/// stay with the statement they continue.
pub fn python_chunks(text: &[u8]) -> Vec<Chunk> {
    let mut starts: Vec<(usize, usize)> = vec![(0, 0)];
    let mut depth = 0usize;
    let mut quote: Option<(u8, bool)> = None; // (quote byte, triple)
    let mut continued = false;
    let mut after_decorator = false;
    let mut row = 0;
    let mut i = 0;
    let mut line_start = true;
    while i < text.len() {
        if line_start {
            line_start = false;
            if quote.is_none() && depth == 0 && !continued {
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
                    } else if text[i..].starts_with(&[q, q, q]) {
                        quote = None;
                        i += 2;
                    }
                } else if byte == b'\n' && !triple {
                    quote = None; // unterminated: let the parser report it
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
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth = depth.saturating_sub(1),
                b'\\' if text.get(i + 1) == Some(&b'\n') => continued = true,
                b'\\' if text[i + 1..].starts_with(b"\r\n") => continued = true,
                _ => {}
            },
        }
        if text.get(i) == Some(&b'\n') {
            row += 1;
            line_start = true;
        }
        i += 1;
    }
    let total_rows = row;
    let mut chunks = Vec::with_capacity(starts.len());
    for (k, &(start, start_row)) in starts.iter().enumerate() {
        let (end, end_row) = starts
            .get(k + 1)
            .copied()
            .unwrap_or((text.len(), total_rows));
        chunks.push(Chunk {
            start,
            end,
            start_row,
            end_row,
        });
    }
    chunks
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

    #[test]
    fn select_merges_adjacent_chunks() {
        let chunks = python_chunks(b"a = 1\nb = 2\nc = 3\nd = 4\n");
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
