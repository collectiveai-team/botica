//! Role execution ports, independent of the provider and artifact transport.

use anyhow::{ensure, Context, Result};
use serde_json::Value;

/// A fresh reviewer context for each role. Only refute receives prior findings.
pub trait Reviewer {
    fn review(&self, role: &str, prior: &[Value]) -> Result<Value>;
}

/// Preserve completed role evidence even when a later role fails.
pub trait ReviewResults {
    fn save(&mut self, role: &str, result: &Value) -> Result<()>;
}

/// Execute exactly the selected roles, with refute last, then validate their accumulated evidence.
pub fn execute(
    plan: &Value,
    reviewer: &impl Reviewer,
    results: &mut impl ReviewResults,
) -> Result<Value> {
    ensure!(
        plan["status"] == "complete",
        "cannot execute an incomplete plan"
    );
    let selected = plan["selected_roles"]
        .as_array()
        .context("missing role selection")?;
    let mut prior = Vec::new();
    for role in selected
        .iter()
        .filter(|role| *role != "refute")
        .chain(selected.iter().filter(|role| *role == "refute"))
    {
        let role = role.as_str().context("invalid role")?;
        let result = reviewer.review(role, if role == "refute" { &prior } else { &[] })?;
        ensure!(
            result["role"] == role
                && result["inputs"] == plan["inputs"]
                && result["base"] == plan["inputs"]["base"]
                && result["head"] == plan["inputs"]["head"],
            "reviewer returned stale or mismatched result"
        );
        results.save(role, &result)?;
        prior.push(result);
    }
    super::results::validate(plan, &prior)
}
