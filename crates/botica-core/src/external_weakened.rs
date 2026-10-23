//! `external-config-weakened`: a value that the settings of a tool set on purpose and that holds
//! less than the fragment of an active external rule asks (`external_config::weakened`). The
//! finding sits on the line of the value, so a comment with a reason there silences it.

use std::ops::Range;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::external_config::Weakening;

pub const EXTERNAL_CONFIG_WEAKENED: &str = "external-config-weakened";

/// One finding for each weakening, on the line of its value in its file, or about the whole file
/// when botica does not find the line. `read` gives the text of a file from the root of the
/// project.
pub fn check(
    weakenings: &[Weakening],
    read: impl Fn(&str) -> Option<String>,
    severity: Severity,
) -> Vec<Diagnostic> {
    weakenings
        .iter()
        .map(|weakening| {
            let message = message(weakening);
            let text = read(&weakening.file);
            match text
                .as_deref()
                .and_then(|text| Some((text, locate(text, weakening)?)))
            {
                Some((text, span)) => on_a_line(&weakening.file, text, span, severity, message),
                None => Diagnostic::file_level(
                    &weakening.file,
                    EXTERNAL_CONFIG_WEAKENED,
                    severity,
                    message,
                ),
            }
        })
        .collect()
}

/// `` `preset = "basic"` in pyproject.toml weakens what `pyrefly-strict` asks for (`strict`)``,
/// and how to keep it.
fn message(weakening: &Weakening) -> String {
    let comment = if shares_the_line(&weakening.file) {
        "a `botica: ignore[external-config-weakened] -- <reason>` comment on that line"
    } else {
        "a `botica: ignore-file[external-config-weakened] -- <reason>` comment on a line of its own"
    };
    format!(
        "`{}` in {} weakens what `{}` asks for (`{}`): give the reason in {comment}, or switch \
         the rule off",
        written(weakening),
        weakening.file,
        weakening.rule,
        bare(&weakening.wanted)
    )
}

/// Whether a comment can follow a value on its line. pytest keeps what follows a value in its INI
/// files as part of the value.
fn shares_the_line(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    !(name.ends_with(".ini") || name == "setup.cfg")
}

/// The value as the file writes it: `preset = "basic"`, `minimumReleaseAge: 0`,
/// `min-release-age=3`, or `ignore = ["DOC"]` for a code of ruff that `ignore` turns off.
fn written(weakening: &Weakening) -> String {
    if assignment(&weakening.found).is_some() {
        return weakening.found.clone();
    }
    let key = last_key(&weakening.key);
    match Format::of(&weakening.file) {
        Format::Toml => format!("{key} = {}", weakening.found),
        Format::Yaml => format!("{key}: {}", weakening.found),
        Format::Ini => format!("{key}={}", bare(&weakening.found)),
    }
}

#[derive(Clone, Copy)]
enum Format {
    Toml,
    Yaml,
    Ini,
}

impl Format {
    fn of(file: &str) -> Self {
        if file.ends_with(".toml") || file.ends_with(".cargo/config") {
            Self::Toml
        } else if file.ends_with(".yaml") || file.ends_with(".yml") {
            Self::Yaml
        } else {
            Self::Ini
        }
    }
}

fn last_key(key: &str) -> &str {
    key.rsplit('.').next().unwrap_or(key)
}

/// A TOML string without its quotes; any other value as it is.
fn bare(value: &str) -> String {
    match format!("v = {value}").parse::<toml::Table>() {
        Ok(table) => match table.get("v") {
            Some(toml::Value::String(text)) => text.clone(),
            _ => value.to_owned(),
        },
        Err(_) => value.to_owned(),
    }
}

/// The key and the first text of the list of `found` when it is a whole assignment, as ruff's
/// `ignore = ["DOC"]` is.
fn assignment(found: &str) -> Option<(String, String)> {
    let table = found.parse::<toml::Table>().ok()?;
    let (key, value) = table.iter().next()?;
    let first = value.as_array()?.first()?.as_str()?;
    Some((key.clone(), first.to_owned()))
}

/// Where the value of `weakening` is in `text`: the selector of ruff in its list, or the key and
/// its value. The first key with that name whose value holds the found value wins, else the first
/// key with that name.
fn locate(text: &str, weakening: &Weakening) -> Option<Range<usize>> {
    if let Some((key, selector)) = assignment(&weakening.found) {
        let after = keys(text, &key).into_iter().next()?.end;
        return ["\"", "'"].iter().find_map(|quote| {
            let quoted = format!("{quote}{selector}{quote}");
            let at = after + text[after..].find(&quoted)?;
            Some(at..at + quoted.len())
        });
    }
    let found = bare(&weakening.found);
    let candidates = keys(text, last_key(&weakening.key));
    let value_of = |key: &Range<usize>| {
        let end = text[key.end..]
            .find('\n')
            .map_or(text.len(), |at| key.end + at);
        let value = &text[key.end..end];
        let value = value
            .find(" #")
            .or_else(|| value.find(" ;"))
            .map_or(value, |at| &value[..at]);
        key.start..key.end + value.trim_end().len()
    };
    candidates
        .iter()
        .map(value_of)
        .find(|span| text[span.clone()].contains(found.as_str()))
        .or_else(|| candidates.first().map(value_of))
}

/// Each place where `key` is written as a key, from the start of the key to the end of its `=`
/// or `:`: at the start of a line, after a `.` of a dotted key, or inside an inline table.
fn keys(text: &str, key: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for (at, _) in text.match_indices(key) {
        let quoted = at > 0 && matches!(bytes[at - 1], b'"' | b'\'');
        let start = if quoted { at - 1 } else { at };
        let mut end = at + key.len();
        if quoted {
            if bytes.get(end) != Some(&bytes[at - 1]) {
                continue;
            }
            end += 1;
        }
        let before = text[..start].trim_end_matches([' ', '\t']);
        let opens = before.is_empty() || before.ends_with(['\n', '.', '{', ',']);
        let rest = &text[end..];
        let gap = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let separator = rest[gap..].chars().next();
        if opens && matches!(separator, Some('=' | ':')) {
            found.push(start..end + gap + 1);
        }
    }
    found
}

fn on_a_line(
    path: &str,
    text: &str,
    span: Range<usize>,
    severity: Severity,
    message: String,
) -> Diagnostic {
    let document = Document::new(path, text);
    let (line, column) = document.position(span.start);
    let (end_line, end_column) = document.position(span.end);
    Diagnostic {
        path: path.to_owned(),
        rule: EXTERNAL_CONFIG_WEAKENED.to_owned(),
        severity,
        start: span.start,
        end: span.end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message,
    }
}
