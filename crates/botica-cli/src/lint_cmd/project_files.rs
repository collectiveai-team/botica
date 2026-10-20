//! What the native rules of project scope share: the files of the project they read, and the paths
//! of their findings seen from where botica runs.

use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::env_schema::SETTINGS_ENV_SCHEMA;
use botica_core::external_config::EXTERNAL_CONFIG;
use botica_core::import_layers::IMPORT_LAYERS;
use botica_core::lockfiles::LOCKFILE_COMMITTED;
use botica_core::{walk, Diagnostic};

use super::{external, import_layers, lockfiles, settings_schema};
use crate::project;

/// The native rules of project scope.
pub const RULES: [&str; 4] = [
    EXTERNAL_CONFIG,
    SETTINGS_ENV_SCHEMA,
    IMPORT_LAYERS,
    LOCKFILE_COMMITTED,
];

/// What the native rules of project scope find, once for the whole project at `root`, shown from
/// the directory `prefix` inside it.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let mut found = external::findings(resolution, root, prefix);
    found.extend(settings_schema::findings(resolution, root, prefix));
    found.extend(import_layers::findings(resolution, root, prefix));
    found.extend(lockfiles::findings(resolution, root, prefix));
    found
}

/// The paths from `root` and the texts of the files that a walk of the project reaches and that
/// `reads` keeps.
pub fn texts(
    resolution: &Arc<Resolution>,
    root: &Path,
    reads: fn(&str) -> bool,
) -> Vec<(String, String)> {
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    walk::files(root, exclusion.as_ref())
        .into_iter()
        .filter(|path| reads(path))
        .filter_map(|path| {
            let text = std::fs::read_to_string(root.join(&path)).ok()?;
            Some((path, text))
        })
        .collect()
}

/// Gives the paths of `found`, from the root of the project, from the directory `prefix` inside
/// it.
pub fn seen_from(prefix: Option<&str>, found: &mut [Diagnostic]) {
    let Some(prefix) = prefix else {
        return;
    };
    for finding in found {
        finding.path = from_working_dir(prefix, &finding.path);
    }
}

/// A path from the root of the project, seen from the directory `prefix` inside it.
fn from_working_dir(prefix: &str, path: &str) -> String {
    let here: Vec<&str> = prefix.split('/').collect();
    let there: Vec<&str> = path.split('/').collect();
    let shared = here
        .iter()
        .zip(&there)
        .take_while(|(a, b)| a == b)
        .count()
        .min(there.len() - 1);
    format!(
        "{}{}",
        "../".repeat(here.len() - shared),
        there[shared..].join("/")
    )
}
