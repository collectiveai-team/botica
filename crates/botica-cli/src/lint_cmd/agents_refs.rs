use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::agents_refs::{check, reads, Source, AGENTS_MD_REFS_EXIST};
use botica_core::{walk, Diagnostic, Severity};

use super::project_files;

/// What `agents-md-refs-exist` finds, once for the whole project: the `@<path>` references of the
/// `AGENTS.md` and `CLAUDE.md` that a walk of the project reaches, against the files on disk, shown
/// from where botica runs.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(AGENTS_MD_REFS_EXIST)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let texts = project_files::texts(resolution, root, reads);
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let exists = |path: &str| root.join(path).exists();
    let ignored = |path: &str| walk::is_gitignored(root, path);
    let mut found = check(&sources, &exists, &ignored, Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}
