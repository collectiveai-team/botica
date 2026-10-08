use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::NativeRule;
use anyhow::Result;
use botica_astgrep::AstGrepSet;
use botica_ext::{ExtensionHost, FileIn};
use std::collections::HashMap;

pub struct Engine {
    native: Vec<Box<dyn NativeRule>>,
    astgrep: AstGrepSet,
}

impl Engine {
    pub fn new(native: Vec<Box<dyn NativeRule>>, astgrep: AstGrepSet) -> Self {
        Self { native, astgrep }
    }

    pub fn astgrep(&self) -> &AstGrepSet {
        &self.astgrep
    }

    /// Lints text the caller already holds. Does no I/O.
    pub fn lint_source(&self, path: &str, text: &str) -> Vec<Diagnostic> {
        let doc = Document::new(path, text);
        let mut out = Vec::new();
        for rule in &self.native {
            rule.check(&doc, &mut out);
        }
        for hit in self.astgrep.check(path.as_ref(), text) {
            let (line, column) = doc.position(hit.start);
            out.push(Diagnostic {
                path: path.to_owned(),
                rule: hit.rule_id,
                severity: Severity::parse(hit.severity),
                start: hit.start,
                end: hit.end,
                line,
                column,
                message: hit.message,
            });
        }
        out.sort_by(|a, b| (a.start, &a.rule).cmp(&(b.start, &b.rule)));
        out
    }
}

pub struct ExtensionRun {
    host: ExtensionHost,
    languages: Vec<String>,
}

pub fn language_of(path: &str) -> Option<&'static str> {
    match path.rsplit_once('.')?.1 {
        "py" | "pyi" => Some("python"),
        "ts" | "tsx" | "mts" | "cts" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        _ => None,
    }
}

impl ExtensionRun {
    pub fn start(command: &[String]) -> Result<Self> {
        let host = ExtensionHost::spawn(command)?;
        let languages = host
            .rules
            .iter()
            .flat_map(|rule| rule.languages.iter().cloned())
            .collect();
        Ok(Self { host, languages })
    }

    pub fn lint<'a>(
        &mut self,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
        batch_size: usize,
    ) -> Result<Vec<Diagnostic>> {
        let wanted: Vec<FileIn<'_>> = files
            .into_iter()
            .filter(|(path, _)| {
                language_of(path).is_some_and(|l| self.languages.iter().any(|x| x == l))
            })
            .map(|(path, text)| FileIn { path, text })
            .collect();
        let mut out = Vec::new();
        for batch in wanted.chunks(batch_size.max(1)) {
            let texts: HashMap<&str, &str> = batch.iter().map(|f| (f.path, f.text)).collect();
            for wire in self.host.lint_files(batch)? {
                let text = texts.get(wire.path.as_str()).copied().unwrap_or("");
                let doc = Document::new(&wire.path, text);
                let (line, column) = doc.position(wire.start);
                out.push(Diagnostic {
                    path: wire.path,
                    rule: wire.rule,
                    severity: Severity::parse(&wire.severity),
                    start: wire.start,
                    end: wire.end,
                    line,
                    column,
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
