use std::fmt;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub offset: u64,
    pub line: u64,
    pub column: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// What a fixture is compared on: where the diagnostic is, how serious it is and what it says.
/// The rule and its code are left out, so the same expectations can judge a replacement rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDiagnostic {
    pub path: String,
    pub severity: String,
    /// `None` for a diagnostic about a whole file (`"range": null`).
    pub range: Option<Range>,
    pub message: String,
}

impl fmt::Display for FixtureDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.range {
            Some(range) => write!(
                f,
                "{}:{}:{}",
                self.path, range.start.line, range.start.column
            )?,
            None => write!(f, "{}", self.path)?,
        }
        write!(f, " {}: {}", self.severity, self.message)
    }
}

impl FixtureDiagnostic {
    pub(crate) fn from_json(diagnostic: &Value) -> Result<Self> {
        Ok(Self {
            path: text(diagnostic, "path")?,
            severity: text(diagnostic, "severity")?,
            range: range(&diagnostic["range"])?,
            message: text(diagnostic, "message")?,
        })
    }
}

fn text(value: &Value, key: &str) -> Result<String> {
    value[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("a diagnostic has no `{key}` text"))
}

fn range(value: &Value) -> Result<Option<Range>> {
    if value.is_null() {
        return Ok(None);
    }
    let start = position(&value["start"]).context("range.start")?;
    let end = position(&value["end"]).context("range.end")?;
    Ok(Some(Range { start, end }))
}

fn position(value: &Value) -> Result<Position> {
    let number = |key: &str| {
        value[key]
            .as_u64()
            .ok_or_else(|| anyhow!("a position has no `{key}` number"))
    };
    Ok(Position {
        offset: number("offset")?,
        line: number("line")?,
        column: number("column")?,
    })
}
