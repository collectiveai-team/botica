//! Reads suppression directives out of the text of one comment (spec §8).

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Line,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    Botica,
    Noqa,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Id {
    pub name: String,
    pub span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Malformed {
    MissingList,
    EmptyList,
    Unclosed,
    UnknownVerb(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Directive {
    Suppress {
        scope: Scope,
        channel: Channel,
        ids: Vec<Id>,
        span: Range<usize>,
        /// Whether `--` and some text follow the directive.
        reason: bool,
    },
    Malformed {
        span: Range<usize>,
        reason: Malformed,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Channels<'a> {
    pub noqa: bool,
    /// What the codes of botica start with in a `noqa` comment: `BT`, and the code of each
    /// catalog of the project.
    pub prefixes: &'a [String],
}

const BOTICA: &str = "botica:";

fn is_separator(byte: u8) -> bool {
    matches!(byte, b',' | b' ' | b'\t' | b'\r')
}

fn starts_a_word(body: &str, at: usize) -> bool {
    at == 0 || matches!(body.as_bytes()[at - 1], b' ' | b'\t' | b'#' | b'\r')
}

/// The earliest marker that starts a word, as `(offset, which)`.
fn next_marker(body: &str, from: usize, channels: Channels<'_>) -> Option<(usize, Channel)> {
    let mut markers = vec![(BOTICA, Channel::Botica)];
    if channels.noqa {
        markers.push(("noqa", Channel::Noqa));
        markers.push(("NOQA", Channel::Noqa));
    }
    markers
        .into_iter()
        .filter_map(|(needle, channel)| {
            body[from..]
                .match_indices(needle)
                .map(|(at, _)| from + at)
                .find(|&at| starts_a_word(body, at))
                .map(|at| (at, channel))
        })
        .min_by_key(|&(at, _)| at)
}

/// Splits `body[from..to]` on commas and blanks into ids with their absolute spans.
fn split_ids(body: &str, from: usize, to: usize, base: usize) -> Vec<Id> {
    let bytes = body.as_bytes();
    let mut ids = Vec::new();
    let mut i = from;
    while i < to {
        if is_separator(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < to && !is_separator(bytes[i]) {
            i += 1;
        }
        ids.push(Id {
            name: body[start..i].to_owned(),
            span: base + start..base + i,
        });
    }
    ids
}

fn skip_blanks(body: &str, from: usize) -> usize {
    let rest = &body[from..];
    from + (rest.len() - rest.trim_start_matches([' ', '\t']).len())
}

/// Whether the text after a directive is `--` followed by a reason.
fn has_reason(rest: &str) -> bool {
    rest.trim_start()
        .strip_prefix("--")
        .is_some_and(|reason| !reason.trim().is_empty())
}

/// Where to resume after a directive: the rest of the comment is a reason when it starts with `--`.
fn after_reason(body: &str, from: usize) -> usize {
    if body[from..].trim_start().starts_with("--") {
        body.len()
    } else {
        from
    }
}

fn list(
    body: &str,
    start: usize,
    bracket: usize,
    scope: Scope,
    base: usize,
    out: &mut Vec<Directive>,
) -> usize {
    let open = bracket + 1;
    let Some(close_at) = body[open..].find(']') else {
        out.push(Directive::Malformed {
            span: base + start..base + body.trim_end().len(),
            reason: Malformed::Unclosed,
        });
        return body.len();
    };
    let close = open + close_at;
    let ids = split_ids(body, open, close, base);
    let span = base + start..base + close + 1;
    out.push(if ids.is_empty() {
        Directive::Malformed {
            span,
            reason: Malformed::EmptyList,
        }
    } else {
        Directive::Suppress {
            scope,
            channel: Channel::Botica,
            ids,
            span,
            reason: has_reason(&body[close + 1..]),
        }
    });
    after_reason(body, close + 1)
}

/// Parses the directive that begins with `botica:` at `start`. Returns where to resume.
fn botica(body: &str, start: usize, base: usize, out: &mut Vec<Directive>) -> usize {
    let after = start + BOTICA.len();
    let word_start = skip_blanks(body, after);
    let word_len = body[word_start..]
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .unwrap_or(body.len() - word_start);
    let word_end = word_start + word_len;
    let word = &body[word_start..word_end];
    let bracket = skip_blanks(body, word_end);
    let has_list = body[bracket..].starts_with('[');
    let malformed = |reason| Directive::Malformed {
        span: base + start..base + word_end,
        reason,
    };
    match (word, has_list) {
        ("ignore", true) => list(body, start, bracket, Scope::Line, base, out),
        ("ignore-file", true) => list(body, start, bracket, Scope::File, base, out),
        ("ignore" | "ignore-file", false) => {
            out.push(malformed(Malformed::MissingList));
            word_end
        }
        (word, true) if !word.is_empty() => {
            out.push(malformed(Malformed::UnknownVerb(word.to_owned())));
            word_end
        }
        _ => after,
    }
}

/// A ruff or flake8 code: uppercase letters, then digits.
fn is_code(token: &str) -> bool {
    let letters = token.bytes().take_while(u8::is_ascii_uppercase).count();
    letters > 0 && letters < token.len() && token[letters..].bytes().all(|b| b.is_ascii_digit())
}

/// Parses `noqa: A1, BTX001 -- reason`. Codes that do not start with one of `prefixes` (`BT`, the
/// codes of the catalogs of the project) belong to other tools.
fn noqa(
    body: &str,
    start: usize,
    base: usize,
    prefixes: &[String],
    out: &mut Vec<Directive>,
) -> usize {
    let after = start + "noqa".len();
    let colon = skip_blanks(body, after);
    if !body[colon..].starts_with(':') {
        return after;
    }
    let bytes = body.as_bytes();
    let mut ids = Vec::new();
    let mut end = colon + 1;
    let mut i = colon + 1;
    loop {
        while i < bytes.len() && is_separator(bytes[i]) {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let token_start = i;
        while i < bytes.len() && !is_separator(bytes[i]) {
            i += 1;
        }
        let token = &body[token_start..i];
        if token.starts_with("--") {
            i = bytes.len();
            break;
        }
        if !is_code(token) {
            i = token_start;
            break;
        }
        if prefixes
            .iter()
            .any(|prefix| token.starts_with(prefix.as_str()))
        {
            ids.push(Id {
                name: token.to_owned(),
                span: base + token_start..base + i,
            });
            end = i;
        }
    }
    if !ids.is_empty() {
        out.push(Directive::Suppress {
            scope: Scope::Line,
            channel: Channel::Noqa,
            ids,
            span: base + start..base + end,
            reason: false,
        });
    }
    i.max(after)
}

/// Reads every directive in the text of one comment that starts at absolute offset `base`.
pub(crate) fn parse(body: &str, base: usize, channels: Channels<'_>, out: &mut Vec<Directive>) {
    let mut cursor = 0;
    while cursor < body.len() {
        let Some((at, channel)) = next_marker(body, cursor, channels) else {
            break;
        };
        cursor = match channel {
            Channel::Botica => botica(body, at, base, out),
            Channel::Noqa => noqa(body, at, base, channels.prefixes, out),
        };
    }
}
