//! What `--diff`, `--staged` and `--diff-file` give a run of `botica lint` (ADR 0009): the
//! changes, the files to lint, and the text of a file before the change.

use super::git;
use super::LintArgs;
use anyhow::{Context, Result};
use botica_core::diff::Changes;
use botica_core::walk::{self, SourceFile};
use botica_core::Engine;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

enum Source {
    /// The merge base of `--diff <rev>` and `HEAD`.
    MergeBase(String),
    Staged,
    Patch,
}

pub(super) struct Diff {
    pub(super) changes: Changes,
    source: Source,
}

impl Diff {
    /// The diff the options ask for, or `None` without one.
    pub(super) fn read(args: &LintArgs) -> Result<Option<Self>> {
        let (changes, source) = if let Some(rev) = &args.diff {
            let base = git::merge_base(rev)?;
            let mut changes = Changes::parse(&git::diff(Some(&base))?)?;
            for path in git::untracked()? {
                let lines = std::fs::read(&path).map_or(0, |bytes| line_count(&bytes));
                changes.add_created(&path, lines);
            }
            (changes, Source::MergeBase(base))
        } else if args.staged {
            (Changes::parse(&git::diff(None)?)?, Source::Staged)
        } else if let Some(patch) = &args.diff_file {
            let text = std::fs::read_to_string(patch)
                .with_context(|| format!("cannot read `{}`", patch.display()))?;
            let changes = Changes::parse(&text)
                .with_context(|| format!("`{}` is not a unified diff", patch.display()))?;
            (changes, Source::Patch)
        } else {
            return Ok(None);
        };
        Ok(Some(Self { changes, source }))
    }

    /// The files of the diff to lint: all of them without `paths`, else the ones under `paths`.
    pub(super) fn files(&self, paths: &[PathBuf], root: &Path) -> Vec<String> {
        let under: Vec<String> = paths
            .iter()
            .map(|path| walk::relative_path(root, path))
            .collect();
        self.changes
            .paths()
            .filter(|path| {
                under.is_empty()
                    || under.iter().any(|dir| {
                        dir.is_empty()
                            || *path == dir.as_str()
                            || path
                                .strip_prefix(dir.trim_end_matches('/'))
                                .is_some_and(|rest| rest.starts_with('/'))
                    })
            })
            .map(str::to_owned)
            .collect()
    }

    /// Whether the files are read from the index rather than from the working tree.
    pub(super) fn is_staged(&self) -> bool {
        matches!(self.source, Source::Staged)
    }

    /// The files as they are staged; a file git cannot show is left out.
    pub(super) fn staged(files: &[String]) -> Vec<SourceFile> {
        files
            .iter()
            .filter_map(|path| {
                git::show("", path).map(|bytes| SourceFile {
                    path: path.clone(),
                    bytes,
                })
            })
            .collect()
    }

    /// The text before the change of each file whose base a native rule reads.
    pub(super) fn bases(
        &self,
        engine: &Engine,
        files: &[String],
    ) -> Result<HashMap<String, String>> {
        let mut bases = HashMap::new();
        for path in files {
            if self.changes.creates(path) || !engine.reads_base(path) {
                continue;
            }
            let base = match &self.source {
                Source::MergeBase(rev) => shown(rev, path),
                Source::Staged => shown("HEAD", path),
                Source::Patch => {
                    let text = std::fs::read_to_string(path)
                        .with_context(|| format!("cannot read `{path}`"))?;
                    self.changes.base(path, &text)?
                }
            };
            if let Some(base) = base {
                bases.insert(path.clone(), base);
            }
        }
        Ok(bases)
    }
}

fn shown(rev: &str, path: &str) -> Option<String> {
    git::show(rev, path).and_then(|bytes| String::from_utf8(bytes).ok())
}

fn line_count(bytes: &[u8]) -> usize {
    let newlines = bytes.iter().filter(|&&byte| byte == b'\n').count();
    newlines + usize::from(bytes.last().is_some_and(|&byte| byte != b'\n'))
}
