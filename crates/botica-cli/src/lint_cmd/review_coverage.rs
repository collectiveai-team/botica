//! `review-coverage-current` over a project: the `REVIEW.md` of each role that the configuration
//! expects, and the hook and job files their blocks cover, read from the working tree.

use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};
use botica_config::{Resolution, ReviewConfig};
use botica_core::review::coverage::role_directory;
use botica_core::review_coverage::{check, missing, Role, REVIEW_COVERAGE_CURRENT};
use botica_core::{Diagnostic, Severity};

use super::project_files;

/// What `review-coverage-current` finds, once for the whole project at `root`, shown from the
/// directory `prefix` inside it.
pub fn findings(
    resolution: &Arc<Resolution>,
    review: &ReviewConfig,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(REVIEW_COVERAGE_CURRENT)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let severity = Severity::from(entry.severity);
    let mut found = match role_directory(review.dir.as_deref(), |dir| root.join(dir).is_dir()) {
        Ok(directory) => roles(review, root, &directory, severity),
        Err(error) => vec![missing(".", &format!("{error:#}"), severity)],
    };
    project_files::seen_from(prefix, &mut found);
    found
}

fn roles(
    review: &ReviewConfig,
    root: &Path,
    directory: &str,
    severity: Severity,
) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    let mut texts = Vec::new();
    for role in review.expected_roles() {
        let path = format!("{directory}/{}/REVIEW.md", role.name());
        match read(root, &path) {
            Ok(Some(bytes)) => texts.push((path, String::from_utf8_lossy(&bytes).into_owned())),
            Ok(None) => found.push(missing(&path, "missing trusted role", severity)),
            Err(error) => found.push(missing(&path, &format!("{error:#}"), severity)),
        }
    }
    let roles: Vec<Role<'_>> = texts
        .iter()
        .map(|(path, text)| Role { path, text })
        .collect();
    let read = |path: &str| read(root, path);
    found.extend(check(&roles, &read, severity));
    found
}

/// The bytes of a regular file of the project, `None` when it is missing. Like the blobs that
/// review check reads, a path with a symbolic link in any of its parts is refused, so that a
/// declared path cannot read outside the project.
fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    let mut full = root.to_path_buf();
    let parts: Vec<&str> = path.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        full.push(part);
        let Ok(metadata) = std::fs::symlink_metadata(&full) else {
            return Ok(None);
        };
        let last = index + 1 == parts.len();
        let kind = metadata.file_type();
        if kind.is_symlink() || (last && !kind.is_file()) || (!last && !kind.is_dir()) {
            bail!("coverage source {path} is not a regular file of the project");
        }
    }
    Ok(Some(std::fs::read(full)?))
}
