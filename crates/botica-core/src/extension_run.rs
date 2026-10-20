//! Running an extension over files: the languages botica knows and the process that serves them.

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use anyhow::{Context, Result};
use botica_ext::{Deadlines, ExtensionHost, FileIn, Selection, WireDiagnostic};
use std::collections::HashMap;
use std::path::Path;

pub struct ExtensionRun {
    host: ExtensionHost,
    languages: Vec<String>,
    /// The rules of a catalog that the process runs, and their options; `None` for `--ext`.
    selection: Option<Selection>,
}

/// Each language that a rule may name, with the extensions of its files.
const LANGUAGES: [(&str, &[&str]); 6] = [
    ("python", &["py", "pyi"]),
    ("typescript", &["ts", "tsx", "mts", "cts"]),
    ("javascript", &["js", "jsx", "mjs", "cjs"]),
    ("go", &["go"]),
    ("rust", &["rs"]),
    ("json", &["json"]),
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
        Ok(Self::started(ExtensionHost::spawn_in(command, dir)?))
    }

    fn started(host: ExtensionHost) -> Self {
        let languages = host
            .rules
            .iter()
            .flat_map(|rule| rule.languages.iter().cloned())
            .collect();
        Self {
            host,
            languages,
            selection: None,
        }
    }

    /// Starts the extension of the rules of a catalog: it must declare the rules of `selection`
    /// and no rule outside `allowed`, the rules that the catalogs run with the same command, with
    /// `deadlines`. Each request names the rules of `selection` with their options.
    pub fn start_for(
        command: &[String],
        dir: Option<&Path>,
        selection: Selection,
        allowed: &[String],
        deadlines: Deadlines,
    ) -> Result<Self> {
        let mut run = Self::started(ExtensionHost::spawn_with(command, dir, deadlines)?);
        run.host.check_declared(&selection.rules, allowed)?;
        run.selection = Some(selection);
        Ok(run)
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
            let found = self.host.lint_files(batch, self.selection.as_ref())?;
            out.extend(found.into_iter().map(|wire| diagnostic(wire, &texts)));
        }
        Ok(out)
    }

    /// Sends `root` and the paths of `files`, as `(path, text)`, in one `lint/project`: the rules
    /// of project scope read the project themselves. The texts give the lines of the findings.
    pub fn lint_project(&mut self, root: &Path, files: &[(&str, &str)]) -> Result<Vec<Diagnostic>> {
        let selection = self
            .selection
            .as_ref()
            .context("`lint/project` asks for the rules of a catalog")?;
        let paths: Vec<&str> = files.iter().map(|(path, _)| *path).collect();
        let texts: HashMap<&str, &str> = files.iter().copied().collect();
        let found = self
            .host
            .lint_project(&root.to_string_lossy(), &paths, selection)?;
        Ok(found
            .into_iter()
            .map(|wire| diagnostic(wire, &texts))
            .collect())
    }

    pub fn finish(self) -> Result<()> {
        self.host.shutdown()
    }
}

/// A finding of the extension, with its lines and columns in the text of its file.
fn diagnostic(wire: WireDiagnostic, texts: &HashMap<&str, &str>) -> Diagnostic {
    let text = texts.get(wire.path.as_str()).copied().unwrap_or("");
    let doc = Document::new(&wire.path, text);
    let (line, column) = doc.position(wire.start);
    let (end_line, end_column) = doc.position(wire.end);
    Diagnostic {
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
    }
}
