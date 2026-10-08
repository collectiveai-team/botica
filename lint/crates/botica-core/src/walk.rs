use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Component, Path, PathBuf};

pub struct SourceFile {
    pub path: String,
    pub text: String,
}

const LINTED_EXTENSIONS: [&str; 6] = ["py", "pyi", "ts", "tsx", "js", "jsx"];

/// One walk of the tree, honouring `.gitignore`, on several threads; sorted, so the result
/// does not depend on thread timing.
pub fn discover(root: &Path) -> Vec<PathBuf> {
    let found = std::sync::Mutex::new(Vec::new());
    WalkBuilder::new(root)
        .hidden(true)
        .build_parallel()
        .run(|| {
            let mut mine = Flush {
                paths: Vec::new(),
                into: &found,
            };
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    let linted = entry.file_type().is_some_and(|t| t.is_file())
                        && entry
                            .path()
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| LINTED_EXTENSIONS.contains(&e));
                    if linted {
                        mine.paths.push(entry.into_path());
                    }
                }
                ignore::WalkState::Continue
            })
        });
    let mut paths = found.into_inner().expect("walk");
    paths.sort_unstable();
    paths
}

/// A walker thread's paths, handed over to the shared list when the thread's visitor drops.
struct Flush<'a> {
    paths: Vec<PathBuf>,
    into: &'a std::sync::Mutex<Vec<PathBuf>>,
}

impl Drop for Flush<'_> {
    fn drop(&mut self) {
        if let Ok(mut all) = self.into.lock() {
            all.append(&mut self.paths);
        }
    }
}

fn read_one(root: &Path, path: &Path) -> Option<SourceFile> {
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
}

pub fn read_all(root: &Path, paths: &[PathBuf]) -> Vec<SourceFile> {
    paths
        .par_iter()
        .filter_map(|path| read_one(root, path))
        .collect()
}

/// Reads and lints each file on the same thread, with no barrier between reading and linting.
/// Returns the texts too, for extensions.
pub fn read_and_lint(
    engine: &Engine,
    root: &Path,
    paths: &[PathBuf],
) -> (Vec<SourceFile>, Vec<Diagnostic>) {
    let (sources, found): (Vec<SourceFile>, Vec<Vec<Diagnostic>>) = paths
        .par_iter()
        .filter_map(|path| {
            let file = read_one(root, path)?;
            let found = engine.lint_source(&file.path, &file.text);
            Some((file, found))
        })
        .unzip();
    let mut out: Vec<Diagnostic> = found.into_iter().flatten().collect();
    out.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
    (sources, out)
}

/// How files are handed to the worker threads. Only `InOrder` is meant for users; the others are
/// measurement aids kept from the spike.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Schedule {
    #[default]
    InOrder,
    LargestFirst,
    /// `InOrder`, plus one timing row per file on stderr.
    Timed,
}

pub fn lint_files(engine: &Engine, files: &[SourceFile], schedule: Schedule) -> Vec<Diagnostic> {
    let mut out: Vec<Diagnostic> = match schedule {
        Schedule::InOrder => files
            .par_iter()
            .flat_map_iter(|file| engine.lint_source(&file.path, &file.text))
            .collect(),
        Schedule::LargestFirst => lint_largest_first(engine, files),
        Schedule::Timed => lint_timed(engine, files),
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

/// One `[file] path<TAB>bytes<TAB>lines<TAB>longest line<TAB>microseconds` row per file on
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
