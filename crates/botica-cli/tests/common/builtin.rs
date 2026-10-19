//! The topics that the binary carries, and a cache that already holds the files they point at.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use botica_catalog::References;

pub fn embedded() -> References {
    References::embedded().expect("the embedded topics load")
}

/// How many external references the built-in topics have, and how many topics.
pub fn counts() -> (usize, usize) {
    let embedded = embedded();
    let references = embedded.topics().iter().map(|t| t.references.len()).sum();
    (embedded.topics().len(), references)
}

/// Fills `cache` with a file for each external reference of the built-in topics, as if they had
/// been fetched: each has the lines the reference names, and its anchor on the first of them.
pub fn fill_cache(cache: &Path) {
    let mut files: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for topic in embedded().topics() {
        for reference in &topic.references {
            let (Some(repo), Some(rev), Some(path)) =
                (&reference.repo, &reference.rev, &reference.path)
            else {
                continue;
            };
            let file = cache.join(repo).join(rev).join(path);
            let lines = files.entry(file).or_default();
            let (first, last) = reference.line_range().unwrap_or((1, 1));
            while lines.len() < last {
                lines.push(format!("line {}", lines.len() + 1));
            }
            if let Some(anchor) = &reference.anchor {
                lines[first - 1] = anchor.clone();
            }
        }
    }
    for (file, lines) in files {
        std::fs::create_dir_all(file.parent().expect("has a parent")).expect("create parents");
        std::fs::write(file, lines.join("\n") + "\n").expect("write the file");
    }
}
