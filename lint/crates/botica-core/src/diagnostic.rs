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
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
