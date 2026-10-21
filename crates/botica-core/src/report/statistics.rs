use std::collections::HashMap;
use std::io::{self, Write};

use serde::Serialize;

use super::{Format, JsonError, Report, SCHEMA_VERSION};

/// Whether `format` has a way to say counts. Only the two that are a plain listing do.
pub fn supports(format: Format) -> bool {
    matches!(format, Format::Text | Format::Json)
}

struct Row<'a> {
    rule: &'a str,
    code: Option<&'a str>,
    count: usize,
}

/// A count per rule, the most frequent first and by name among equals.
fn rows<'a>(report: &'a Report<'a>) -> Vec<Row<'a>> {
    let codes: HashMap<&str, &str> = report
        .rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule.code.as_str()))
        .collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for diagnostic in report.diagnostics {
        *counts.entry(diagnostic.rule.as_str()).or_default() += 1;
    }
    let mut rows: Vec<Row<'a>> = counts
        .into_iter()
        .map(|(rule, count)| Row {
            rule,
            code: codes.get(rule).copied(),
            count,
        })
        .collect();
    rows.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.rule.cmp(b.rule)));
    rows
}

#[derive(Serialize)]
struct JsonRow<'a> {
    rule: &'a str,
    code: Option<&'a str>,
    count: usize,
}

#[derive(Serialize)]
struct JsonStatistics<'a> {
    schema_version: u32,
    statistics: Vec<JsonRow<'a>>,
    errors: Vec<JsonError<'a>>,
}

/// Writes the counts instead of the findings. The caller checks `supports` first.
pub fn write(report: &Report<'_>, format: Format, out: &mut impl Write) -> io::Result<()> {
    let rows = rows(report);
    match format {
        Format::Json => {
            let document = JsonStatistics {
                schema_version: SCHEMA_VERSION,
                statistics: rows
                    .iter()
                    .map(|row| JsonRow {
                        rule: row.rule,
                        code: row.code,
                        count: row.count,
                    })
                    .collect(),
                errors: report.errors.iter().map(JsonError::from).collect(),
            };
            serde_json::to_writer(&mut *out, &document)?;
            writeln!(out)
        }
        Format::Text => rows.iter().try_for_each(|row| {
            writeln!(
                out,
                "{:>7}  {:<12} {}",
                row.count,
                row.code.unwrap_or("-"),
                row.rule
            )
        }),
        Format::Github | Format::Sarif | Format::Markdown => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "counts are only available in the text and json formats",
        )),
    }
}
