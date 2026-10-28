use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::botica_version::{check, reads, Param, Source, BOTICA_VERSION_ALIGNED};
use botica_core::{Diagnostic, Severity};

use super::project_files;
use super::recipe_conflict::ProjectRecipes;

/// What `botica-version-aligned` finds, once for the whole project: the versions of botica that
/// `prek.toml`, `.pre-commit-config.yaml`, the workflows of `.github/workflows/` and the parameter
/// `botica-version` of the configuration name, shown from where botica runs.
pub fn findings(
    resolution: &Arc<Resolution>,
    recipes: &ProjectRecipes,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(BOTICA_VERSION_ALIGNED)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let texts = project_files::texts(resolution, root, reads);
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let config = recipes.botica_version.as_ref().map(|value| {
        let text = std::fs::read_to_string(root.join(&recipes.config_file)).unwrap_or_default();
        (value, text)
    });
    let param = config.as_ref().map(|(value, text)| Param {
        path: &recipes.config_file,
        text,
        value,
    });
    let mut found = check(&sources, param.as_ref(), Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}
