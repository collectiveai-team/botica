use crate::diagnostic::{Diagnostic, Severity};
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};

mod markdown;
mod sarif;
pub mod statistics;

/// Version of the JSON document. A consumer may rely on the keys of one version.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// One line per diagnostic. The failures of the run are not part of it: the caller says them
    /// on stderr.
    Text,
    Json,
    /// Workflow commands of GitHub Actions: one annotation per diagnostic and per error.
    Github,
    /// SARIF 2.1.0, with the failures of the run as notifications of the invocation.
    Sarif,
    /// A Markdown table, one row per diagnostic, for the summary of a job of GitHub Actions.
    Markdown,
}

/// What an output format says about a rule of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleDoc {
    pub id: String,
    pub code: String,
    /// The standard the rule implements (`CES-71`), if it names one.
    pub standard: Option<String>,
    pub summary: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Extension,
    /// A file the walk found and could not read.
    Read,
}

impl ErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Extension => "extension",
            Self::Read => "read",
        }
    }
}

/// A failure of the run itself, apart from the findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError {
    pub kind: ErrorKind,
    /// The command of the extension that failed.
    pub extension: Option<String>,
    /// The file that could not be read.
    pub path: Option<String>,
    pub message: String,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.extension, &self.path) {
            (Some(subject), _) | (None, Some(subject)) => {
                write!(f, "{} `{subject}`: {}", self.kind.as_str(), self.message)
            }
            (None, None) => f.write_str(&self.message),
        }
    }
}

pub struct Report<'a> {
    pub diagnostics: &'a [Diagnostic],
    pub errors: &'a [RunError],
    pub rules: &'a [RuleDoc],
    pub tool_version: &'a str,
}

pub fn write(report: &Report<'_>, format: Format, out: &mut impl Write) -> io::Result<()> {
    match format {
        Format::Text => text(report, out),
        Format::Json => json(report, out),
        Format::Github => github(report, out),
        Format::Sarif => sarif::write(report, out),
        Format::Markdown => markdown::write(report, out),
    }
}

fn escape(text: &str, in_property: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '%' => escaped.push_str("%25"),
            '\r' => escaped.push_str("%0D"),
            '\n' => escaped.push_str("%0A"),
            ':' if in_property => escaped.push_str("%3A"),
            ',' if in_property => escaped.push_str("%2C"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn annotation_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info | Severity::Hint => "notice",
    }
}

fn github(report: &Report<'_>, out: &mut impl Write) -> io::Result<()> {
    for d in report.diagnostics {
        write!(
            out,
            "::{} file={}",
            annotation_level(d.severity),
            escape(&d.path, true)
        )?;
        if !d.file_level {
            write!(
                out,
                ",line={},col={},endLine={},endColumn={}",
                d.line, d.column, d.end_line, d.end_column
            )?;
        }
        writeln!(
            out,
            ",title={}::{}",
            escape(&format!("botica ({})", d.rule), true),
            escape(&d.message, false)
        )?;
    }
    for error in report.errors {
        writeln!(
            out,
            "::error title=botica::{}",
            escape(&error.to_string(), false)
        )?;
    }
    Ok(())
}

fn text(report: &Report<'_>, out: &mut impl Write) -> io::Result<()> {
    report
        .diagnostics
        .iter()
        .try_for_each(|diagnostic| writeln!(out, "{diagnostic}"))
}

#[derive(Serialize)]
struct Point {
    offset: usize,
    line: usize,
    column: usize,
}

#[derive(Serialize)]
struct Range {
    start: Point,
    end: Point,
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    rule: &'a str,
    code: Option<&'a str>,
    standard: Option<&'a str>,
    severity: &'static str,
    path: &'a str,
    range: Option<Range>,
    message: &'a str,
}

#[derive(Serialize)]
struct JsonError<'a> {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    message: &'a str,
}

impl<'a> From<&'a RunError> for JsonError<'a> {
    fn from(error: &'a RunError) -> Self {
        Self {
            kind: error.kind.as_str(),
            extension: error.extension.as_deref(),
            path: error.path.as_deref(),
            message: &error.message,
        }
    }
}

struct Diagnostics<'a> {
    items: &'a [Diagnostic],
    rules: HashMap<&'a str, &'a RuleDoc>,
}

impl Serialize for Diagnostics<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.items.iter().map(|d| {
            let doc = self.rules.get(d.rule.as_str());
            JsonDiagnostic {
                rule: &d.rule,
                code: doc.map(|rule| rule.code.as_str()),
                standard: doc.and_then(|rule| rule.standard.as_deref()),
                severity: d.severity.as_str(),
                path: &d.path,
                range: (!d.file_level).then_some(Range {
                    start: Point {
                        offset: d.start,
                        line: d.line,
                        column: d.column,
                    },
                    end: Point {
                        offset: d.end,
                        line: d.end_line,
                        column: d.end_column,
                    },
                }),
                message: &d.message,
            }
        }))
    }
}

#[derive(Serialize)]
struct JsonReport<'a> {
    schema_version: u32,
    diagnostics: Diagnostics<'a>,
    errors: Vec<JsonError<'a>>,
}

fn json(report: &Report<'_>, out: &mut impl Write) -> io::Result<()> {
    let document = JsonReport {
        schema_version: SCHEMA_VERSION,
        diagnostics: Diagnostics {
            items: report.diagnostics,
            rules: report
                .rules
                .iter()
                .map(|rule| (rule.id.as_str(), rule))
                .collect(),
        },
        errors: report.errors.iter().map(JsonError::from).collect(),
    };
    serde_json::to_writer(&mut *out, &document)?;
    writeln!(out)
}
