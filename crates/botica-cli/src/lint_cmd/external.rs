use std::path::Path;
use std::sync::Arc;

use botica_catalog::Kind;
use botica_config::Resolution;
use botica_core::external_config::{check, Requirement, EXTERNAL_CONFIG};
use botica_core::{walk, Diagnostic, Severity};

use crate::project;

/// What `external-config` finds, once for the whole project: the files at the root of the project
/// that lack the configuration of an active external rule whose tool the project uses, shown from
/// where botica runs.
pub fn findings(
    resolution: &Arc<Resolution>,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(EXTERNAL_CONFIG)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let active: Vec<_> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .filter(|entry| {
            entry
                .rule
                .implementations
                .iter()
                .any(|implementation| implementation.kind == Kind::External)
        })
        .collect();
    if active.is_empty() {
        return Vec::new();
    }
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    let files = walk::files(root, exclusion.as_ref());
    let requirements: Vec<Requirement<'_>> = active
        .iter()
        .flat_map(|entry| Requirement::of_used(&entry.rule, Some(&files)))
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
