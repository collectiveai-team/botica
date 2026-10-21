mod lexer;

use super::{merge, narrow, Chunk, Split};
use lexer::js_split;

/// Splits JavaScript/TypeScript/TSX source into top-level statements. Strings end at a newline
/// and regex literals too, so JSX text with a stray quote only costs splits or a `suspect` flag,
/// not correctness: a wrong split leaves a partial tree with errors, and the caller then parses
/// the whole file. Unterminated strings are not flagged, since JSX text holds stray quotes.
pub(super) fn js_chunks(text: &[u8]) -> Split {
    js_split(text, 0, text.len(), 0, false)
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
