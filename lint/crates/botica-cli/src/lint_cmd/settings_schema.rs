use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::env_schema::{check, reads, Source, SCHEMA, SETTINGS_ENV_SCHEMA};
use botica_core::{walk, Diagnostic, Severity};

use crate::project;

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
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    let texts: Vec<(String, String)> = walk::files(root, exclusion.as_ref())
        .into_iter()
        .filter(|path| reads(path))
        .filter_map(|path| {
            let text = std::fs::read_to_string(root.join(&path)).ok()?;
            Some((path, text))
        })
        .collect();
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let mut found = check(&schema, &sources, Severity::from(entry.severity));
    if let Some(prefix) = prefix {
        for finding in &mut found {
            finding.path = from_working_dir(prefix, &finding.path);
        }
    }
    found
}

/// A path from the root of the project, seen from the directory `prefix` inside it.
fn from_working_dir(prefix: &str, path: &str) -> String {
    match path
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        Some(inside) => inside.to_owned(),
        None => format!("{}{path}", "../".repeat(prefix.split('/').count())),
    }
}
