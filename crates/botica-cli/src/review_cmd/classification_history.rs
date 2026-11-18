use super::tier;
use anyhow::{ensure, Result};
use botica_core::history::{self, RuleEntry, RunLine};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::SystemTime;

pub(super) fn record(assessment: &tier::Assessment, report: &Value) -> Result<()> {
    let started = SystemTime::now();
    let detail = json!({"mode":"shadow","inputs":report["inputs"],"recommendation":report["recommendation"],
        "human_required":report["human_required"],"automation_eligible":report["automation_eligible"],
        "merge_performed":false,"probabilities":report["probabilities"],"thresholds":report["thresholds"],
        "thresholds_calibrated":false,"human_response":null});
    ensure!(
        detail.to_string().len() <= 64 * 1024,
        "classification history detail exceeds 64 KiB"
    );
    let hash = blake3::hash(&serde_json::to_vec(
        &json!({"model":botica_core::review::classifier::MODEL,"thresholds":report["thresholds"]}),
    )?)
    .to_hex()
    .to_string();
    let mut rules = BTreeMap::new();
    rules.insert(
        "classifier".into(),
        RuleEntry {
            hits: usize::from(report["automation_eligible"] != true),
            hash: Some(hash.clone()),
            ..RuleEntry::default()
        },
    );
    let run = RunLine {
        v: 1,
        run_id: history::run_id(started),
        started_at: history::unix_seconds(started),
        commit: Some(assessment.git.head.clone()),
        head: Some(assessment.git.head.clone()),
        branch: None,
        botica: env!("CARGO_PKG_VERSION").into(),
        rules_fingerprint: hash,
        key_id: None,
        mode: "classify".into(),
        origin: "classify".into(),
        rules,
        detail: Some(detail),
    };
    history::append(&assessment.git.root, &run, &[])
}
