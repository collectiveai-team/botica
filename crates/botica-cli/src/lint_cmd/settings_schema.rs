use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::env_schema::{check, reads, Source, SCHEMA, SETTINGS_ENV_SCHEMA};
use botica_core::{Diagnostic, Severity};

use super::project_files;

/// What `settings-env-schema` finds, once for the whole project: the schema at the root of the
/// project against the files a walk of the project reaches, shown from where botica runs. Nothing
/// when the project has no schema.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(SETTINGS_ENV_SCHEMA)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let Ok(schema) = std::fs::read_to_string(root.join(SCHEMA)) else {
        return Vec::new();
    };
    let texts = project_files::texts(resolution, root, reads);
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let mut found = check(&schema, &sources, Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}
