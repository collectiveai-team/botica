use std::path::Path;

use botica_config::Resolution;
use botica_core::external_config::{check, Requirement, EXTERNAL_CONFIG};
use botica_core::{Diagnostic, Severity};

/// What `external-config` finds, once for the whole project: the files at the root of the project
/// that lack the configuration of an active external rule, shown from where botica runs.
pub fn findings(resolution: &Resolution, root: &Path, prefix: Option<&str>) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(EXTERNAL_CONFIG)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let requirements: Vec<Requirement<'_>> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .flat_map(|entry| Requirement::of(&entry.rule))
        .collect();
    let mut found = check(
        &requirements,
        |target| std::fs::read_to_string(root.join(target)).ok(),
        Severity::from(entry.severity),
    );
    if let Some(prefix) = prefix {
        let up = "../".repeat(prefix.split('/').count());
        for finding in &mut found {
            finding.path = format!("{up}{}", finding.path);
        }
    }
    found
}
