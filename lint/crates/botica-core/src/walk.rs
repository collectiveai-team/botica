use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Component, Path, PathBuf};

pub struct SourceFile {
    pub path: String,
    pub text: String,
}

const LINTED_EXTENSIONS: [&str; 6] = ["py", "pyi", "ts", "tsx", "js", "jsx"];

/// One walk of the tree, honouring `.gitignore`.
pub fn discover(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(true)
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .map(ignore::DirEntry::into_path)
        .filter(|path| {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| LINTED_EXTENSIONS.contains(&e))
        })
        .collect()
}

pub fn read_all(root: &Path, paths: &[PathBuf]) -> Vec<SourceFile> {
    paths
        .par_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(path).ok()?;
            let relative = path.strip_prefix(root).unwrap_or(path);
            let relative: PathBuf = relative
                .components()
                .filter(|c| !matches!(c, Component::CurDir))
                .collect();
            Some(SourceFile {
                path: relative.to_string_lossy().into_owned(),
                text,
            })
        })
        .collect()
}

pub fn lint_files(engine: &Engine, files: &[SourceFile]) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = files
        .par_iter()
        .flat_map_iter(|file| engine.lint_source(&file.path, &file.text))
        .collect();
    out.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
    out
}
