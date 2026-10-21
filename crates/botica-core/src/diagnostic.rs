use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn parse(name: &str) -> Self {
        match name {
            "error" => Self::Error,
            "warning" => Self::Warning,
            "info" => Self::Info,
            _ => Self::Hint,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

impl From<botica_catalog::Severity> for Severity {
    fn from(level: botica_catalog::Severity) -> Self {
        match level {
            botica_catalog::Severity::Error => Self::Error,
            botica_catalog::Severity::Warning => Self::Warning,
            botica_catalog::Severity::Info => Self::Info,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: String,
    pub rule: String,
    pub severity: Severity,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    /// Where the finding ends, 1-based and in characters like `line` and `column`.
    #[serde(default)]
    pub end_line: usize,
    #[serde(default)]
    pub end_column: usize,
    /// About the whole file, with no line of its own (`range: null`): only `ignore-file` silences it.
    #[serde(default)]
    pub file_level: bool,
    pub message: String,
    /// What a rule measured in a function, which the baseline compares with the value it holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<Measure>,
}

/// A value that a rule measures in one function, such as its cognitive complexity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Measure {
    /// The qualified name of the function: `Order::total`.
    pub name: String,
    pub value: u32,
}

impl Diagnostic {
    /// A finding about a whole file: it sits at 1:1 and carries no range of its own.
    pub fn file_level(path: &str, rule: &str, severity: Severity, message: String) -> Self {
        Self {
            path: path.to_owned(),
            rule: rule.to_owned(),
            severity,
            start: 0,
            end: 0,
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
            file_level: true,
            message,
            measure: None,
        }
    }
}

impl fmt::Display for Diagnostic {
    /// A finding about a whole change, which has no path, shows as `(change)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            return write!(
                f,
                "(change): {} [{}] {}",
                self.severity.as_str(),
                self.rule,
                self.message
            );
        }
        write!(
            f,
            "{}:{}:{}: {} [{}] {}",
            self.path,
            self.line,
            self.column,
            self.severity.as_str(),
            self.rule,
            self.message
        )
    }
}
