//! `no-coverage-exclusion`: what the configuration of coverage.py leaves out of its report, in
//! `pyproject.toml`, `.coveragerc` and `setup.cfg`, and a `fail_under` lower than before the
//! change.

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};
use serde::Deserialize;
use std::ops::Range;
use toml::Spanned;

pub const NO_COVERAGE_EXCLUSION: &str = "no-coverage-exclusion";

#[derive(Debug, Default)]
pub struct NoCoverageExclusion;

/// The settings that leave code out of the report; `omit` and `exclude_lines` replace a default,
/// `exclude_also` adds to it.
const LISTS: [&str; 3] = ["omit", "exclude_lines", "exclude_also"];

struct Entry {
    key: &'static str,
    value: String,
    span: Range<usize>,
}

#[derive(Default)]
struct Settings {
    entries: Vec<Entry>,
    fail_under: Option<(f64, Range<usize>)>,
}

impl NativeRule for NoCoverageExclusion {
    fn id(&self) -> &'static str {
        NO_COVERAGE_EXCLUSION
    }

    fn needs(&self) -> Needs {
        Needs::Text
    }

    fn reads_base(&self) -> bool {
        true
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let Some(text) = doc.text() else { return };
        let Some(settings) = settings_of(doc.path, text) else {
            return;
        };
        for entry in &settings.entries {
            out.push(finding(
                doc,
                &entry.span,
                format!(
                    "`{}` leaves `{}` out of the coverage report: cover that code with a test, or \
                     delete it if nothing calls it. For code that cannot run under test, keep the \
                     entry and silence it with the reason.",
                    entry.key, entry.value
                ),
            ));
        }
        let before = doc
            .base()
            .and_then(|base| settings_of(doc.path, base))
            .and_then(|base| base.fail_under);
        if let (Some((now, span)), Some((was, _))) = (&settings.fail_under, before) {
            if *now < was {
                out.push(finding(
                    doc,
                    span,
                    format!(
                        "`fail_under` goes down from {was} to {now}: keep {was}, and cover the new \
                         code with tests or delete what nothing calls."
                    ),
                ));
            }
        }
    }
}

fn finding(doc: &Document<'_>, span: &Range<usize>, message: String) -> Diagnostic {
    let (line, column) = doc.position(span.start);
    let (end_line, end_column) = doc.position(span.end);
    Diagnostic {
        path: doc.path.to_owned(),
        rule: NO_COVERAGE_EXCLUSION.to_owned(),
        severity: Severity::Warning,
        start: span.start,
        end: span.end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        message,
    }
}

/// The settings of coverage.py in the file, or `None` when coverage.py does not read the file.
fn settings_of(path: &str, text: &str) -> Option<Settings> {
    match path.rsplit('/').next().unwrap_or(path) {
        "pyproject.toml" => Some(from_toml(text)),
        ".coveragerc" => Some(from_ini(
            text,
            &["run", "report", "coverage:run", "coverage:report"],
        )),
        "setup.cfg" => Some(from_ini(text, &["coverage:run", "coverage:report"])),
        _ => None,
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Pyproject {
    tool: Tool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Tool {
    coverage: Coverage,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Coverage {
    run: Section,
    report: Section,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Section {
    omit: Vec<Spanned<String>>,
    exclude_lines: Vec<Spanned<String>>,
    exclude_also: Vec<Spanned<String>>,
    fail_under: Option<Spanned<f64>>,
}

/// A `pyproject.toml` that is not valid TOML, or whose settings of coverage have another type,
/// has none: coverage.py fails on it before it measures anything.
fn from_toml(text: &str) -> Settings {
    let Ok(pyproject) = toml::from_str::<Pyproject>(text) else {
        return Settings::default();
    };
    let Coverage { run, report } = pyproject.tool.coverage;
    let mut settings = Settings::default();
    for section in [run, report] {
        let lists = [section.omit, section.exclude_lines, section.exclude_also];
        for (key, values) in LISTS.into_iter().zip(lists) {
            settings
                .entries
                .extend(values.into_iter().map(|value| Entry {
                    key,
                    span: value.span(),
                    value: value.into_inner(),
                }));
        }
        if let Some(value) = section.fail_under {
            settings.fail_under = Some((*value.get_ref(), value.span()));
        }
    }
    settings.entries.sort_by_key(|entry| entry.span.start);
    settings
}

/// The INI of `.coveragerc` and `setup.cfg`, as configparser reads it: a value goes on over the
/// indented lines below its key, and a line that starts with `#` or `;` is a comment.
fn from_ini(text: &str, sections: &[&str]) -> Settings {
    let mut settings = Settings::default();
    let mut in_section = false;
    let mut list: Option<&'static str> = None;
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if let Some(key) = list {
                push_values(&mut settings, text, key, trimmed);
            }
            continue;
        }
        list = None;
        if let Some(name) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            in_section = sections.contains(&name.trim());
            continue;
        }
        let Some(separator) = line.find(['=', ':']).filter(|_| in_section) else {
            continue;
        };
        let key = line[..separator].trim();
        let value = line[separator + 1..].trim();
        if let Some(name) = LISTS
            .into_iter()
            .find(|name| key.eq_ignore_ascii_case(name))
        {
            list = Some(name);
            push_values(&mut settings, text, name, value);
        } else if key.eq_ignore_ascii_case("fail_under") {
            settings.fail_under = value
                .parse()
                .ok()
                .map(|number| (number, span_in(text, value)));
        }
    }
    settings
}

/// The values of a line of a list: `omit` takes paths separated by commas, the others one regular
/// expression per line.
fn push_values(settings: &mut Settings, text: &str, key: &'static str, line: &str) {
    let values: Vec<&str> = if key == "omit" {
        line.split(',').map(str::trim).collect()
    } else {
        vec![line]
    };
    settings.entries.extend(
        values
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(|value| Entry {
                key,
                value: value.to_owned(),
                span: span_in(text, value),
            }),
    );
}

/// Where `part`, a slice of `text`, sits in it.
fn span_in(text: &str, part: &str) -> Range<usize> {
    let start = part.as_ptr() as usize - text.as_ptr() as usize;
    start..start + part.len()
}
