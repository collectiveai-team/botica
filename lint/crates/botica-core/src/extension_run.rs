//! Running an extension over files: the languages botica knows and the process that serves them.

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use anyhow::Result;
use botica_ext::{ExtensionHost, FileIn};
use std::collections::HashMap;
use std::path::Path;

pub struct ExtensionRun {
    host: ExtensionHost,
    languages: Vec<String>,
}

/// Each language that a rule may name, with the extensions of its files.
const LANGUAGES: [(&str, &[&str]); 3] = [
    ("python", &["py", "pyi"]),
    ("typescript", &["ts", "tsx", "mts", "cts"]),
    ("javascript", &["js", "jsx", "mjs", "cjs"]),
];

pub fn language_of(path: &str) -> Option<&'static str> {
    let extension = path.rsplit_once('.')?.1;
    LANGUAGES
        .iter()
        .find(|(_, extensions)| extensions.contains(&extension))
        .map(|(language, _)| *language)
}

/// The extensions of the files of `language`, none for a language botica does not know.
pub fn extensions_of(language: &str) -> &'static [&'static str] {
    LANGUAGES
        .iter()
        .find(|(name, _)| *name == language)
        .map_or(&[], |(_, extensions)| extensions)
}

impl ExtensionRun {
    pub fn start(command: &[String]) -> Result<Self> {
        Self::start_in(command, None)
    }

    /// Starts the extension with `dir` as its working directory (botica's own when `None`).
    pub fn start_in(command: &[String], dir: Option<&Path>) -> Result<Self> {
        let host = ExtensionHost::spawn_in(command, dir)?;
        let languages = host
            .rules
            .iter()
            .flat_map(|rule| rule.languages.iter().cloned())
            .collect();
        Ok(Self { host, languages })
    }

    /// Sends the files in the languages that the extension says it reads.
    pub fn lint<'a>(
        &mut self,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
        batch_size: usize,
    ) -> Result<Vec<Diagnostic>> {
        let wanted: Vec<(&str, &str)> = files
            .into_iter()
            .filter(|(path, _)| {
                language_of(path).is_some_and(|l| self.languages.iter().any(|x| x == l))
            })
            .collect();
        self.lint_all(wanted, batch_size)
    }

    /// Sends every file given: the caller has chosen them, as the catalog does for a rule.
    pub fn lint_all<'a>(
        &mut self,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
        batch_size: usize,
    ) -> Result<Vec<Diagnostic>> {
        let wanted: Vec<FileIn<'_>> = files
            .into_iter()
            .map(|(path, text)| FileIn { path, text })
            .collect();
        let mut out = Vec::new();
        for batch in wanted.chunks(batch_size.max(1)) {
            let texts: HashMap<&str, &str> = batch.iter().map(|f| (f.path, f.text)).collect();
            for wire in self.host.lint_files(batch)? {
                let text = texts.get(wire.path.as_str()).copied().unwrap_or("");
                let doc = Document::new(&wire.path, text);
                let (line, column) = doc.position(wire.start);
                let (end_line, end_column) = doc.position(wire.end);
                out.push(Diagnostic {
                    path: wire.path,
                    rule: wire.rule,
                    severity: Severity::parse(&wire.severity),
                    start: wire.start,
                    end: wire.end,
                    line,
                    column,
                    end_line,
                    end_column,
                    file_level: false,
                    message: wire.message,
                });
            }
        }
        Ok(out)
    }

    pub fn finish(self) -> Result<()> {
        self.host.shutdown()
    }
}
