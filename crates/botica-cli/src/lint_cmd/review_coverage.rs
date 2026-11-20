//! `review-coverage-current` over a project: the `REVIEW.md` of each role that the configuration
//! expects, and the hook and job files their blocks cover, read from the working tree.

use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};
use botica_config::{Resolution, ReviewConfig};
use botica_core::review_coverage::{check, Role, REVIEW_COVERAGE_CURRENT};
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
    let Some(directory) = directory(review, root) else {
        return Vec::new();
    };
    let texts: Vec<(String, String)> = review
        .expected_roles()
        .iter()
        .filter_map(|role| {
            let path = format!("{directory}/{}/REVIEW.md", role.name());
            let bytes = read(root, &path).ok()??;
            Some((path, String::from_utf8_lossy(&bytes).into_owned()))
        })
        .collect();
    let roles: Vec<Role<'_>> = texts
        .iter()
        .map(|(path, text)| Role { path, text })
        .collect();
    let read = |path: &str| read(root, path);
    let mut found = check(&roles, &read, Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}

/// The folder of the roles as `botica review check` picks it: `review.dir`, else `.gitlab/review`
/// when only it exists, else `.github/review`. `None` when both exist, which review check refuses.
fn directory(review: &ReviewConfig, root: &Path) -> Option<String> {
    if let Some(directory) = &review.dir {
        return Some(directory.clone());
    }
    let github = root.join(".github/review").is_dir();
    let gitlab = root.join(".gitlab/review").is_dir();
    match (github, gitlab) {
        (true, true) => None,
        (false, true) => Some(".gitlab/review".to_owned()),
        _ => Some(".github/review".to_owned()),
    }
}

/// The bytes of a regular file of the project, `None` when it is missing. Like the blobs that
/// review check reads, a symbolic link is refused.
fn read(root: &Path, path: &str) -> Result<Option<Vec<u8>>> {
    let full = root.join(path);
    let Ok(metadata) = std::fs::symlink_metadata(&full) else {
        return Ok(None);
    };
    if !metadata.file_type().is_file() {
        bail!("coverage source {path} is not a regular file");
    }
    Ok(Some(std::fs::read(full)?))
}
