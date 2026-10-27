//! `botica recipe check`: the fragments of configuration that the external rules of a recipe need.

use std::path::Path;

use botica_config::Reason;
use botica_core::external_config::{check, Requirement};
use botica_core::Severity;

use crate::recipes_collection::Collection;

/// One line for each fragment that the external implementations of the rule `key` declare and
/// whose tool the project uses, its files from the root being `files`: the file, and why the file
/// does not hold it, if it does not. A rule that is not in the catalog has none, and neither does
/// one whose `python` does not hold the Python of the project (ADR 0015). The same reading as
/// `external-config` decides, whether or not the rule is active.
pub fn of_rule(
    collection: &Collection,
    key: &str,
    root: &Path,
    files: &[String],
) -> Vec<(String, Option<String>)> {
    let Some(entry) = collection.project.resolution.entry(key) else {
        return Vec::new();
    };
    if matches!(entry.reason, Reason::OutsidePython { .. }) {
        return Vec::new();
    }
    Requirement::of_used(&entry.rule, Some(files))
        .iter()
        .map(|requirement| {
            let found = check(
                std::slice::from_ref(requirement),
                |target| std::fs::read_to_string(root.join(target)).ok(),
                Severity::Error,
            );
            let id = format!("{} `{}`", requirement.target, requirement.rule);
            (id, found.into_iter().next().map(|d| d.message))
        })
        .collect()
}
