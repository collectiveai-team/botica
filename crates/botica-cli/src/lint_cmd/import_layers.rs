use std::path::Path;
use std::sync::Arc;

use botica_config::Resolution;
use botica_core::import_layers::{check, reads, Source, IMPORT_LAYERS};
use botica_core::{Diagnostic, Severity};

use super::project_files;

/// What `import-layers` finds, once for the whole project: the imports of the Python files that a
/// walk of the project reaches against the layers of the configuration, shown from where botica
/// runs. Nothing when the configuration names no layer.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution.entry(IMPORT_LAYERS).filter(|entry| entry.active) else {
        return Vec::new();
    };
    let layers: Vec<String> = entry
        .options
        .get("layers")
        .and_then(toml::Value::as_array)
        .map(|layers| {
            layers
                .iter()
                .filter_map(|layer| layer.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if layers.is_empty() {
        return Vec::new();
    }
    let texts = project_files::texts(resolution, root, reads);
    let sources: Vec<Source<'_>> = texts
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    let mut found = check(&layers, &sources, Severity::from(entry.severity));
    project_files::seen_from(prefix, &mut found);
    found
}
