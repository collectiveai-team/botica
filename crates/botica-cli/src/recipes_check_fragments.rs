//! `botica recipe check`: the fragments of configuration that the external rules of a recipe need.

use std::path::Path;

use botica_core::external_config::{check, Requirement};
use botica_core::Severity;

use crate::recipes_collection::Collection;

/// One line for each fragment that the external implementations of the rule `key` declare: the
/// file, and why the file does not hold it, if it does not. A rule that is not in the catalog has
/// none. The same reading as `external-config` decides, whether or not the rule is active.
pub fn of_rule(collection: &Collection, key: &str, root: &Path) -> Vec<(String, Option<String>)> {
    let Some(entry) = collection.project.resolution.entry(key) else {
        return Vec::new();
    };
    Requirement::of(&entry.rule)
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
