use super::{LintArgs, Setup};
use crate::rules;
use anyhow::Result;
use botica_core::cache::{lint_files_cached, Cache};
use botica_core::registry::IO_ERROR;
use botica_core::walk::{self, SourceFile, Unreadable, Walked};
use botica_core::{Diagnostic, Engine, Severity};
use std::path::{Path, PathBuf};

/// What linting the paths produced, before the extension and the post-process.
pub(super) struct Linted {
    /// The texts of the files read; empty when nobody needs them (no extension, no cache).
    pub(super) sources: Vec<SourceFile>,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) io_errors: Vec<Diagnostic>,
    /// Files the walk found and could not read.
    pub(super) unreadable: Vec<Unreadable>,
    pub(super) files_read: usize,
}

/// What the paths of the command line lead to: the files named (or found by a walk that only
/// lists), the findings of the walks that also lint, and the paths that cannot be opened.
struct Reached {
    /// Files still to be linted.
    named: Vec<PathBuf>,
    pub(super) io_errors: Vec<Diagnostic>,
    /// What the directories walked and linted in one pass brought.
    walked: Walked,
}

fn reach(args: &LintArgs, setup: &Setup, root: &Path, overlapped: bool) -> Reached {
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let mut reached = Reached {
        named: Vec::new(),
        io_errors: Vec::new(),
        walked: Walked::default(),
    };
    for path in &paths {
        if args.force_exclude && named_and_excluded(&setup.engine, root, path) {
            continue;
        }
        if !path.is_dir() {
            match std::fs::File::open(path) {
                Ok(_) => reached.named.push(path.clone()),
                Err(error) if setup.io_error_active => {
                    reached.io_errors.push(io_error(&named(root, path), &error))
                }
                Err(_) => {}
            }
        } else {
            let asked_for = if overlapped {
                let walked = walk::walk_and_lint(&setup.engine, root, path);
                reached.walked.read += walked.read;
                reached.walked.diagnostics.extend(walked.diagnostics);
                reached.walked.unreadable.extend(walked.unreadable);
                walked.asked_for
            } else {
                let listed = walk::discover(path, root, &setup.engine);
                reached.named.extend(listed.paths);
                reached.walked.unreadable.extend(listed.unreadable);
                listed.asked_for
            };
            if setup.engine.has_inclusion() {
                let more = walk::walk_asked_for(path, root, &setup.engine, &asked_for);
                reached.named.extend(more.paths);
                reached.walked.unreadable.extend(more.unreadable);
            }
        }
    }
    reached
}

/// Where the result cache lives, if there is one: the directory asked for, or `.botica/cache`
/// under the root of the project (`.botica` is a directory of tools: the walk never enters it).
fn cache_dir(args: &LintArgs, root: &Path) -> Option<PathBuf> {
    match (&args.cache_dir, args.cache) {
        (Some(dir), _) => Some(dir.clone()),
        (None, true) => Some(root.join(".botica").join("cache")),
        (None, false) => None,
    }
}

pub(super) fn lint_paths(args: &LintArgs, setup: &Setup, root: &Path) -> Result<Linted> {
    let cache_dir = cache_dir(args, &setup.project_root);
    let overlapped = cache_dir.is_none() && args.ext.is_none();
    let reached = reach(args, setup, root, overlapped);
    let engine = &setup.engine;
    let io_errors = reached.io_errors;
    if overlapped {
        let named = walk::lint_streaming(engine, root, &reached.named);
        let mut diagnostics = named.diagnostics;
        diagnostics.extend(reached.walked.diagnostics);
        diagnostics.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
        Ok(Linted {
            sources: Vec::new(),
            diagnostics,
            io_errors,
            unreadable: merged(named.unreadable, reached.walked.unreadable),
            files_read: named.read + reached.walked.read,
        })
    } else if let Some(dir) = &cache_dir {
        let (sources, unreadable) = walk::read_all(engine, root, &reached.named);
        let unreadable = merged(unreadable, reached.walked.unreadable);
        let fingerprint = rules::fingerprint(
            setup.with_astgrep,
            args.extra_rules.as_deref(),
            &setup.config_key,
        )?;
        let mut cache = Cache::load(dir, &fingerprint);
        let (diagnostics, stats) = lint_files_cached(engine, &sources, &mut cache);
        cache.save()?;
        if args.timings {
            eprintln!(
                "[timing] cache: {} hits, {} misses",
                stats.hits, stats.misses
            );
        }
        Ok(Linted {
            sources,
            diagnostics,
            io_errors,
            unreadable,
            files_read: 0,
        })
    } else {
        let (sources, walked) = walk::read_and_lint(engine, root, &reached.named);
        Ok(Linted {
            sources,
            diagnostics: walked.diagnostics,
            io_errors,
            unreadable: merged(walked.unreadable, reached.walked.unreadable),
            files_read: 0,
        })
    }
}

/// Both lists, sorted by path.
fn merged(mut first: Vec<Unreadable>, second: Vec<Unreadable>) -> Vec<Unreadable> {
    first.extend(second);
    first.sort_by(|a, b| a.path.cmp(&b.path));
    first
}

/// Whether `extend-exclude` leaves out a path the user named. `.` and the like are never excluded.
fn named_and_excluded(engine: &Engine, root: &Path, path: &Path) -> bool {
    let named = walk::relative_path(root, path);
    !named.is_empty() && engine.skips_path(&named)
}

/// A path the user typed, as the walk would name it (from the root, no `./`), with the trailing
/// `/` the user gave a directory.
fn named(root: &Path, path: &Path) -> String {
    let mut name = walk::relative_path(root, path);
    if path.to_string_lossy().ends_with('/') && !name.ends_with('/') {
        name.push('/');
    }
    name
}

fn io_error(path: &str, error: &std::io::Error) -> Diagnostic {
    Diagnostic::file_level(path, IO_ERROR, Severity::Error, error.to_string())
}
