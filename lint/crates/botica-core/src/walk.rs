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
    let mut out: Vec<Diagnostic> = if std::env::var_os("BOTICA_FILE_TIMES").is_some() {
        lint_timed(engine, files)
    } else if std::env::var_os("BOTICA_SCHED_LARGEST_FIRST").is_some() {
        lint_largest_first(engine, files)
    } else {
        files
            .par_iter()
            .flat_map_iter(|file| engine.lint_source(&file.path, &file.text))
            .collect()
    };
    out.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
    out
}

/// Longest-processing-time-first: rayon cannot split one file, so a few 5k-line files picked up
/// last leave threads idle. Sorting by size and handing out one file at a time bounds the tail.
fn lint_largest_first(engine: &Engine, files: &[SourceFile]) -> Vec<Diagnostic> {
    let mut order: Vec<&SourceFile> = files.iter().collect();
    order.sort_unstable_by_key(|file| std::cmp::Reverse(file.text.len()));
    order
        .par_iter()
        .with_max_len(1)
        .flat_map_iter(|file| engine.lint_source(&file.path, &file.text))
        .collect()
}

/// Experiment: one `path<TAB>bytes<TAB>lines<TAB>longest line<TAB>microseconds` row per file on
/// stderr, to see which files dominate the cost.
fn lint_timed(engine: &Engine, files: &[SourceFile]) -> Vec<Diagnostic> {
    let rows: Vec<(String, Vec<Diagnostic>)> = files
        .par_iter()
        .map(|file| {
            let start = std::time::Instant::now();
            let found = engine.lint_source(&file.path, &file.text);
            let micros = start.elapsed().as_micros();
            let longest = file.text.lines().map(str::len).max().unwrap_or(0);
            let row = format!(
                "{}\t{}\t{}\t{}\t{}",
                file.path,
                file.text.len(),
                file.text.lines().count(),
                longest,
                micros
            );
            (row, found)
        })
        .collect();
    let mut out = Vec::new();
    for (row, found) in rows {
        eprintln!("[file] {row}");
        out.extend(found);
    }
    out
}
