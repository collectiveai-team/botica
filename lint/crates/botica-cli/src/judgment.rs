//! What the judges read of the judgment rules of a project: a judgment rule that an active
//! deterministic rule covers is left out of the exports.

use botica_catalog::{Kind, Rule};
use botica_config::Resolution;

/// The active rules of the project that `covered-by` of the judgment rule names, by slug.
pub fn covering<'a>(resolution: &'a Resolution, rule: &'a Rule) -> Vec<&'a str> {
    rule.implementations
        .iter()
        .filter(|implementation| implementation.kind == Kind::Judgment)
        .flat_map(|implementation| &implementation.covered_by)
        .filter(|slug| resolution.entry(slug).is_some_and(|entry| entry.active))
        .map(String::as_str)
        .collect()
}

/// Whether the judges read the rule: an active judgment rule that no active rule covers.
pub fn exported(resolution: &Resolution, rule: &Rule, active: bool) -> bool {
    active && rule.is_judgment() && covering(resolution, rule).is_empty()
}
