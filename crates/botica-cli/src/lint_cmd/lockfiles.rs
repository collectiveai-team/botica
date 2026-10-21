use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::lockfiles::{check, reads, Source, LOCKFILE_COMMITTED};
use botica_core::{walk, Diagnostic, Severity};

use super::project_files;
use crate::project;

/// What `lockfile-committed` finds, once for the whole project: the manifests that a walk of the
/// project reaches against the lockfiles it reaches, shown from where botica runs.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(LOCKFILE_COMMITTED)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    let paths = walk::files(root, exclusion.as_ref());
    let texts: Vec<(&str, String)> = paths
        .iter()
        .filter(|path| reads(path))
        .filter_map(|path| {
            Some((
                path.as_str(),
                std::fs::read_to_string(root.join(path)).ok()?,
            ))
        })
        .collect();
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let mut found = check(&paths, &sources, Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}
