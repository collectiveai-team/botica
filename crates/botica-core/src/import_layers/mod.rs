//! `import-layers`: the modules of a layer import nothing of a layer above it, directly or through
//! modules that are in no layer, and the independent modules of one layer import nothing of each
//! other. The layers come from the configuration, from the top one down; Python is the language
//! whose imports are read.

mod graph;
mod layers;
mod python;

use std::collections::HashMap;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use graph::Graph;
use layers::{Breach, Layers};

pub const IMPORT_LAYERS: &str = "import-layers";

/// A file of the project, from its root, and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// Whether the rule reads `path`, so a caller only loads those files.
pub fn reads(path: &str) -> bool {
    path.ends_with(".py") || path.ends_with(".pyi")
}

/// What the rule says of the import of the first module of `chain` by `importer`.
fn message(importer: &str, chain: &[String], breach: &Breach<'_>) -> String {
    let what = match chain {
        [target] => format!("`{importer}` imports `{target}`"),
        _ => format!(
            "`{importer}` reaches `{}` through {importer} -> {}",
            chain.last().map_or("", String::as_str),
            chain.join(" -> ")
        ),
    };
    let why = match breach {
        Breach::Above { from, to } => format!(
            "`{to}` is a layer above `{from}`: move what it needs to `{from}` or below, or invert \
             the dependency"
        ),
        Breach::Sibling { from, to } => format!(
            "`{from}` and `{to}` are independent modules of one layer: move what both need to a \
             layer below them"
        ),
    };
    format!("{what}, and {why}")
}

/// The imports of the modules of `sources` that break `layers`, one finding for each import
/// statement, at the statement. Paths are those of the sources.
pub fn check(layers: &[String], sources: &[Source<'_>], severity: Severity) -> Vec<Diagnostic> {
    let layers = Layers::parse(layers);
    if layers.is_empty() {
        return Vec::new();
    }
    let graph = Graph::build(&layers, sources);
    let mut chains = HashMap::new();
    let mut found = Vec::new();
    for module in &graph.modules {
        let Some(from) = layers.place_of(&module.name) else {
            continue;
        };
        for (import, targets) in &module.imports {
            let breach = targets
                .iter()
                .filter(|target| **target != module.name)
                .find_map(|target| {
                    chains
                        .entry((from, target.clone()))
                        .or_insert_with(|| graph.breach_from(&layers, from, target))
                        .clone()
                });
            let Some((chain, breach)) = breach else {
                continue;
            };
            let document = Document::new(module.source.path, module.source.text);
            let (line, column) = document.position(import.start);
            let (end_line, end_column) = document.position(import.end);
            found.push(Diagnostic {
                path: module.source.path.to_owned(),
                rule: IMPORT_LAYERS.to_owned(),
                severity,
                start: import.start,
                end: import.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                measure: None,
                message: message(&module.name, &chain, &breach),
            });
        }
    }
    found.sort_by(|a, b| (&a.path, a.start).cmp(&(&b.path, b.start)));
    found
}
