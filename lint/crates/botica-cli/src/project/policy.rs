use std::sync::Arc;

use botica_config::Resolution;
use botica_core::report::RuleDoc;
use botica_core::suppress::{Policy, INVALID, UNUSED};
use botica_core::Severity;

/// What the suppression comments may name, and whether the rules about them report.
pub fn suppression_policy(resolution: &Arc<Resolution>) -> Policy {
    let mut policy = Policy::new().with_compat_ast_grep_ignore(resolution.compat_ast_grep_ignore());
    for entry in resolution.entries() {
        let spellings: Vec<&str> = std::iter::once(entry.rule.code.as_str())
            .chain(entry.rule.aliases.iter().map(String::as_str))
            .collect();
        policy = policy.with_rule(&entry.rule.id, &spellings);
    }
    let external = Arc::clone(resolution);
    policy = policy.with_external(move |id| external.is_external(id));
    let reporting = |slug: &str| {
        resolution
            .entry(slug)
            .filter(|entry| entry.active)
            .map(|entry| Severity::from(entry.severity))
    };
    if let Some(level) = reporting(UNUSED) {
        policy = policy.report_unused(level);
    }
    if let Some(level) = reporting(INVALID) {
        policy = policy.report_invalid(level);
    }
    policy
}

/// The catalog's rules as the output formats describe them.
pub fn rule_docs(resolution: &Resolution) -> Vec<RuleDoc> {
    resolution
        .entries()
        .iter()
        .map(|entry| RuleDoc {
            id: entry.rule.id.clone(),
            code: entry.rule.code.clone(),
            summary: entry.rule.summary.clone(),
            severity: Severity::from(entry.severity),
        })
        .collect()
}
