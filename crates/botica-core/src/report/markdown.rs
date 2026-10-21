use std::collections::HashMap;
use std::io::{self, Write};

use super::Report;

/// A cell of a Markdown table: `|` escaped and the lines joined.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace(['\r', '\n'], " ")
}

/// One row per finding, with its severity, rule, place and message, then the failures of the run
/// as a list; a run with neither says so in a line.
pub(super) fn write(report: &Report<'_>, out: &mut impl Write) -> io::Result<()> {
    if report.diagnostics.is_empty() && report.errors.is_empty() {
        return writeln!(out, "No findings.");
    }
    let codes: HashMap<&str, &str> = report
        .rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule.code.as_str()))
        .collect();
    if !report.diagnostics.is_empty() {
        writeln!(out, "| Severity | Rule | File | Finding |")?;
        writeln!(out, "|---|---|---|---|")?;
    }
    for d in report.diagnostics {
        let rule = match codes.get(d.rule.as_str()) {
            Some(code) => format!("`{}` ({code})", d.rule),
            None => format!("`{}`", d.rule),
        };
        let place = match (d.path.is_empty(), d.file_level) {
            (true, _) => "(change)".to_owned(),
            (false, true) => format!("`{}`", d.path),
            (false, false) => format!("`{}:{}`", d.path, d.line),
        };
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            d.severity.as_str(),
            cell(&rule),
            cell(&place),
            cell(&d.message)
        )?;
    }
    if !report.errors.is_empty() {
        writeln!(out, "\nThe run failed:\n")?;
        for error in report.errors {
            writeln!(out, "- {}", cell(&error.to_string()))?;
        }
    }
    Ok(())
}
