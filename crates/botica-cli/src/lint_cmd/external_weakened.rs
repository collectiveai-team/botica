use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use botica_catalog::Kind;
use botica_config::Resolution;
use botica_core::external_config::{weakened, Requirement};
use botica_core::external_weakened::{check, EXTERNAL_CONFIG_WEAKENED};
use botica_core::{walk, Findings, Severity};

use super::project_files;
use crate::project;

/// What `external-config-weakened` finds, once for the whole project: the values of the settings
/// of a tool that hold less than an active external rule asks and that no comment with a reason
/// silences, and the comments about it that silence nothing, shown from where botica runs. The
/// findings that a comment silenced come apart.
pub fn findings(resolution: &Arc<Resolution>, root: &Path, prefix: Option<&str>) -> Findings {
    let Some(entry) = resolution
        .entry(EXTERNAL_CONFIG_WEAKENED)
        .filter(|entry| entry.active)
    else {
        return Findings::default();
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
        return Findings::default();
    }
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    let files = walk::files(root, exclusion.as_ref());
    let requirements: Vec<Requirement<'_>> = active
        .iter()
        .flat_map(|entry| Requirement::of_used(&entry.rule, Some(&files)))
        .collect();
    let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
    let mut found = check(
        &weakened(&requirements, read),
        read,
        Severity::from(entry.severity),
    );
    let mut paths: BTreeSet<String> = found.iter().map(|finding| finding.path.clone()).collect();
    paths.extend(
        requirements
            .iter()
            .map(|requirement| requirement.target.to_owned()),
    );
    let policy = project::project_pass_policy(resolution);
    let mut kept = Vec::new();
    let mut silenced = Vec::new();
    for path in paths {
        let (mut here, rest): (Vec<_>, Vec<_>) =
            found.into_iter().partition(|finding| finding.path == path);
        found = rest;
        if let Some(text) = read(&path) {
            silenced.extend(policy.report_project(&path, &text, &mut here));
        }
        kept.extend(here);
    }
    project_files::seen_from(prefix, &mut kept);
    project_files::seen_from(prefix, &mut silenced);
    Findings {
        found: kept,
        silenced,
    }
}
