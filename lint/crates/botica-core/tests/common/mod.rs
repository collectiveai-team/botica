#![allow(dead_code)]

use botica_astgrep::AstGrepSet;
use botica_core::suppress::Policy;
use botica_core::{Diagnostic, Document, Engine, NativeRule, Severity};

/// Reports every `FLAG` it finds, on the line where it starts.
pub struct Flag;

impl NativeRule for Flag {
    fn id(&self) -> &'static str {
        "flag-rule"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        for (start, _) in text.match_indices("FLAG") {
            let (line, column) = doc.position(start);
            let (end_line, end_column) = doc.position(start + 4);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Error,
                start,
                end: start + 4,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: "flagged".to_owned(),
            });
        }
    }
}

/// Reports once per file, with no line.
pub struct WholeFile;

impl NativeRule for WholeFile {
    fn id(&self) -> &'static str {
        "whole-file"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        if text.contains("BIG") {
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Warning,
                start: 0,
                end: 0,
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                file_level: true,
                message: "too big".to_owned(),
            });
        }
    }
}

/// Reports from `BEGIN` to the next `END`, which may be several lines apart.
pub struct Multi;

impl NativeRule for Multi {
    fn id(&self) -> &'static str {
        "multi-rule"
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let text = doc
            .text()
            .expect("the engine gives text to rules that need it");
        for (start, _) in text.match_indices("BEGIN") {
            let Some(length) = text[start..].find("END") else {
                continue;
            };
            let (line, column) = doc.position(start);
            let (end_line, end_column) = doc.position(start + length + 3);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: self.id().to_owned(),
                severity: Severity::Error,
                start,
                end: start + length + 3,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: "spans lines".to_owned(),
            });
        }
    }
}

pub fn policy() -> Policy {
    Policy::new()
        .with_rule("flag-rule", &["BTFLG001", "old-flag"])
        .with_rule("whole-file", &["BTFLG002"])
        .with_rule("other-rule", &["BTFLG003"])
        .with_rule("multi-rule", &["BTFLG004"])
        .report_unused(Severity::Warning)
        .report_invalid(Severity::Warning)
}

pub fn engine(policy: Policy) -> Engine {
    Engine::new(
        vec![Box::new(Flag), Box::new(WholeFile), Box::new(Multi)],
        AstGrepSet::default(),
    )
    .with_suppressions(policy)
}

/// `rule@line` of every diagnostic, in order.
pub fn found(path: &str, text: &str) -> Vec<String> {
    summarize(&engine(policy()).lint_source(path, text))
}

pub fn summarize(found: &[Diagnostic]) -> Vec<String> {
    found
        .iter()
        .map(|d| format!("{}@{}", d.rule, d.line))
        .collect()
}
