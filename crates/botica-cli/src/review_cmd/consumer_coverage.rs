use anyhow::{Context, Result};
use botica_core::review::coverage;
use botica_core::review::roles::RoleDefinition;
use serde_json::{json, Value};

use super::super::tier;
#[path = "consumer_rules.rs"]
mod rules;

pub(super) fn inspect(
    assessment: &tier::Assessment,
    role: &RoleDefinition,
    text: &str,
) -> Result<Value> {
    let declarations = role
        .covered
        .as_ref()
        .context("unverified legacy coverage: declare covered metadata")?;
    let mut references = rules::references(assessment, &declarations.rules)?;
    references.extend(coverage::check_references(declarations, |file| {
        assessment.git.blob(file)
    })?);
    let expected = coverage::render(&mut references)?;
    let actual = coverage::current_block(text)?;
    Ok(
        json!({"status": if actual.as_deref() == Some(&expected) { "current" } else { "stale" },
        "expected_block": expected, "references": references}),
    )
}
