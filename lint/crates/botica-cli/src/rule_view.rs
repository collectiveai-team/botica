use std::collections::BTreeMap;

use botica_catalog::{Severity, Status};
use botica_config::{Entry, IgnoreOrigin};
use serde::Serialize;

#[derive(Serialize)]
pub struct IgnoreView<'a> {
    pub glob: &'a str,
    pub origin: IgnoreOrigin,
}

/// One rule as the configuration resolved it, for `botica rules --json`. The CES of the rule is
/// documentation, not part of this public format.
#[derive(Serialize)]
pub struct RuleView<'a> {
    pub rule: &'a str,
    pub code: &'a str,
    pub family: Option<&'a str>,
    pub summary: &'a str,
    pub status: Status,
    pub active: bool,
    /// Why it is or is not active, for people: the wording may change, `active` will not.
    pub reason: String,
    pub severity: Severity,
    pub languages: Vec<&'a str>,
    pub aliases: &'a [String],
    pub tags: &'a [String],
    pub ignores: Vec<IgnoreView<'a>>,
    pub options: &'a BTreeMap<String, toml::Value>,
}

impl<'a> RuleView<'a> {
    pub fn new(entry: &'a Entry) -> Self {
        let rule = &entry.rule;
        Self {
            rule: &rule.id,
            code: &rule.code,
            family: rule.family.as_deref(),
            summary: &rule.summary,
            status: rule.status,
            active: entry.active,
            reason: entry.reason.to_string(),
            severity: entry.severity,
            languages: rule.languages(),
            aliases: &rule.aliases,
            tags: &rule.tags,
            ignores: entry
                .ignore_patterns
                .iter()
                .map(|pattern| IgnoreView {
                    glob: &pattern.glob,
                    origin: pattern.origin,
                })
                .collect(),
            options: &entry.options,
        }
    }
}
