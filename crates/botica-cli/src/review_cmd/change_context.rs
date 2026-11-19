use super::tier;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn classification(
    assessment: &tier::Assessment,
    plan: &Value,
    limit: usize,
) -> Result<Value> {
    let mut changes = Vec::new();
    let mut remaining = limit;
    for selection in botica_core::review::state::diff_selections(plan)? {
        let mut arguments = vec![
            "diff".to_owned(),
            "--no-ext-diff".into(),
            "--no-textconv".into(),
            "--no-renames".into(),
            selection.from.clone(),
            selection.to.clone(),
            "--".into(),
        ];
        arguments.extend(
            selection
                .paths
                .iter()
                .map(|path| format!(":(top,literal){path}")),
        );
        let bytes = if selection.paths.is_empty() {
            Vec::new()
        } else {
            assessment.git.read_limited(
                &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
                remaining,
            )?
        };
        remaining = remaining.saturating_sub(bytes.len());
        changes.push(json!({"from":selection.from,"to":selection.to,
            "diff":String::from_utf8(bytes).context("diff is not UTF-8")?}));
    }
    Ok(json!(changes))
}

pub(super) fn collect(assessment: &tier::Assessment, plan: &Value, limit: usize) -> Result<Value> {
    let mut pairs = BTreeSet::new();
    for comparison in botica_core::review::risk_comparisons(&plan["risk"]) {
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
