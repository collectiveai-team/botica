//! Region parsing: parse only the top-level statements that hold a prefilter candidate.
//!
//! A rule's match lies inside one top-level statement (rules that look at siblings or at the
//! module itself are excluded by the caller), so the other statements need not be parsed.
//! tree-sitter's included ranges keep byte offsets global. Statement boundaries come from a
//! minimal lexer per language (Python, and JavaScript/TypeScript/TSX). Inside a top-level class
//! or function, `members` narrows further to the header and the members that hold a candidate.

mod js;
mod python;
#[cfg(test)]
mod tests;

use ast_grep_language::SupportLang;

/// The top-level statements of `text`, or `None` for a language without a splitter.
pub fn chunks(language: SupportLang, text: &[u8]) -> Option<Split> {
    match language {
        SupportLang::Python => Some(python::python_chunks(text)),
        SupportLang::JavaScript | SupportLang::TypeScript | SupportLang::Tsx => {
            Some(js::js_chunks(text))
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

/// Narrows the picked top-level statements to class/function members, for languages that can.
pub fn members(
    language: SupportLang,
    text: &[u8],
    picked: &[Chunk],
    positions: &[usize],
) -> Vec<Chunk> {
    match language {
        SupportLang::Python => python::python_members(text, picked, positions),
        SupportLang::JavaScript | SupportLang::TypeScript | SupportLang::Tsx => {
            js::js_members(text, picked, positions)
        }
        _ => merge(picked.to_vec()),
    }
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

/// Pushes `chunk` narrowed to `header`, the `members` holding a position and `tail`; or whole,
/// if it has fewer than two members or a position outside them (the match may be the container).
fn narrow(
    chunk: Chunk,
    header: Chunk,
    members: &[Chunk],
    tail: Option<Chunk>,
    positions: &[usize],
    out: &mut Vec<Chunk>,
) {
    let (Some(first), Some(last)) = (members.first(), members.last()) else {
        out.push(chunk);
        return;
    };
    let inside: Vec<usize> = positions
        .iter()
        .copied()
        .filter(|&p| p >= chunk.start && p < chunk.end)
        .collect();
    if members.len() < 2 || inside.iter().any(|&p| p < first.start || p >= last.end) {
        out.push(chunk);
        return;
    }
    out.push(header);
    out.extend(pick(members, &inside));
    out.extend(tail);
}

/// The chunks holding at least one of `positions` (sorted), each once, in order.
pub fn pick(chunks: &[Chunk], positions: &[usize]) -> Vec<Chunk> {
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
