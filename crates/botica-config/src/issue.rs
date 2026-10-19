use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIssue {
    pub file: Option<PathBuf>,
    pub location: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub issues: Vec<ConfigIssue>,
}

impl ConfigError {
    pub(crate) fn new(issues: Vec<ConfigIssue>) -> Self {
        Self { issues }
    }

    pub(crate) fn single(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(vec![issue(location, message)])
    }

    pub fn in_file(mut self, path: &Path) -> Self {
        for issue in &mut self.issues {
            issue.file.get_or_insert_with(|| path.to_path_buf());
        }
        self
    }
}

pub(crate) fn issue(location: impl Into<String>, message: impl Into<String>) -> ConfigIssue {
    ConfigIssue {
        file: None,
        location: location.into(),
        message: message.into(),
    }
}

impl fmt::Display for ConfigIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}: ", file.display())?;
        }
        if !self.location.is_empty() {
            write!(f, "{}: ", self.location)?;
        }
        write!(f, "{}", self.message)
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(f, "{issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}
