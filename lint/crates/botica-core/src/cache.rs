use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use crate::walk::SourceFile;
use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "lint-cache.json";

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    fingerprint: String,
    entries: HashMap<String, Vec<Diagnostic>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
}

/// Per-file results keyed by `blake3(path, text)`. The whole cache is discarded when the rule
/// set changes (`fingerprint`), and only files seen in the latest run are kept.
pub struct Cache {
    file: PathBuf,
    stored: Stored,
}

impl Cache {
    pub fn load(dir: &Path, fingerprint: &str) -> Self {
        let file = dir.join(FILE_NAME);
        let stored = std::fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Stored>(&bytes).ok())
            .filter(|stored| stored.fingerprint == fingerprint)
            .unwrap_or_else(|| Stored {
                fingerprint: fingerprint.to_owned(),
                entries: HashMap::new(),
            });
        Self { file, stored }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file, serde_json::to_vec(&self.stored)?)
            .with_context(|| format!("cannot write {}", self.file.display()))
    }
}

fn key(file: &SourceFile) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(file.path.as_bytes());
    hasher.update(&[0]);
    hasher.update(&file.bytes);
    hasher.finalize().to_hex().to_string()
}

pub fn lint_files_cached(
    engine: &Engine,
    files: &[SourceFile],
    cache: &mut Cache,
) -> (Vec<Diagnostic>, CacheStats) {
    let known = &cache.stored.entries;
    let results: Vec<(String, Vec<Diagnostic>, bool)> = files
        .par_iter()
        .map(|file| {
            let key = key(file);
            match known.get(&key) {
                Some(found) => (key, found.clone(), true),
                None => {
                    let found = engine.lint_bytes(&file.path, &file.bytes);
                    (key, found, false)
                }
            }
        })
        .collect();
    let hits = results.iter().filter(|(_, _, hit)| *hit).count();
    let stats = CacheStats {
        hits,
        misses: results.len() - hits,
    };
    let mut diagnostics = Vec::new();
    let mut entries = HashMap::with_capacity(results.len());
    for (key, found, _) in results {
        diagnostics.extend(found.iter().cloned());
        entries.insert(key, found);
    }
    cache.stored.entries = entries;
    diagnostics.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
    (diagnostics, stats)
}
