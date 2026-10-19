//! The `.gitignore` of the project, as `botica recipe check` and `install` read it for the
//! `gitignore` step.

use std::fs;
use std::path::Path;

const FILE: &str = ".gitignore";

/// What the root of the project has as `.gitignore`.
pub enum Gitignore {
    Missing,
    Text(String),
    /// Why it cannot be used: it is not a file, or cannot be read.
    Unusable(String),
}

impl Gitignore {
    pub fn read(root: &Path) -> Self {
        let full = root.join(FILE);
        if fs::symlink_metadata(&full).is_err() {
            return Self::Missing;
        }
        if !full.is_file() {
            return Self::Unusable(format!("{FILE} is not a file"));
        }
        match fs::read_to_string(&full) {
            Ok(text) => Self::Text(text),
            Err(error) => Self::Unusable(format!("cannot read {FILE}: {error}")),
        }
    }
}

/// Whether the text has `line` as a line, ignoring white space at its end.
pub fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|candidate| candidate.trim_end() == line)
}

/// Whether the text has `line` as a whole line, ignoring white space around it.
pub fn has_literal(text: &str, line: &str) -> bool {
    text.lines().any(|candidate| candidate.trim() == line)
}
