use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogIssue {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    pub issues: Vec<CatalogIssue>,
}

impl fmt::Display for CatalogIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl fmt::Display for CatalogError {
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

impl std::error::Error for CatalogError {}

pub(crate) fn issue(path: impl Into<String>, message: impl Into<String>) -> CatalogIssue {
    CatalogIssue {
        path: path.into(),
        message: message.into(),
    }
}
