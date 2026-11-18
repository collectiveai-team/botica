use super::tier;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn collect(assessment: &tier::Assessment, plan: &Value, limit: usize) -> Result<Value> {
    let mut pairs = BTreeSet::new();
    for name in ["branch", "integration"] {
        for comparison in plan["risk"][name]["comparisons"]
            .as_array()
            .into_iter()
            .flatten()
        {
            anyhow::ensure!(
                comparison["evidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .all(|file| file["kind"] == "text"),
                "non-text change context is unavailable"
            );
            pairs.insert((
                comparison["from"]
                    .as_str()
                    .context("missing comparison base")?,
                comparison["to"]
                    .as_str()
                    .context("missing comparison result")?,
            ));
        }
    }
    let mut changes = Vec::new();
    let mut remaining = limit;
    for (from, to) in pairs {
        let bytes = assessment.git.read_limited(
            &[
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                from,
                to,
                "--",
            ],
            remaining,
        )?;
        remaining = remaining.saturating_sub(bytes.len());
        changes.push(json!({"from":from,"to":to,"diff":String::from_utf8(bytes).context("diff is not UTF-8")?}));
    }
    Ok(json!(changes))
}
