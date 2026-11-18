use super::tier;
use anyhow::{ensure, Result};
use botica_core::history::{self, RunLine};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::SystemTime;

pub(super) fn record(assessment: &tier::Assessment, report: &Value) -> Result<()> {
    append(&assessment.git.root, report, "classify")
}

pub(super) fn append(root: &Path, report: &Value, origin: &str) -> Result<()> {
    let head = report["inputs"]["head"]
        .as_str()
        .filter(|head| {
            matches!(head.len(), 40 | 64) && head.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .ok_or_else(|| anyhow::anyhow!("handoff has no full head object ID"))?;
    let started = SystemTime::now();
    let reasons: Vec<_> = report["reasons"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|reason| {
            json!({"code":reason["code"],"source":reason["source"],
            "probability":reason["probability"]})
        })
        .collect();
    let detail = json!({"mode":report["mode"],"observation_only":report["observation_only"],
        "deterministic_tier":report["deterministic_tier"],"effective_tier":report["effective_tier"],
        "inputs":report["inputs"],"recommendation":report["recommendation"],
        "human_required":report["human_required"],"automation_eligible":report["automation_eligible"],
        "merge_performed":false,"probabilities":report["probabilities"],"thresholds":report["thresholds"],
        "question_digest":report["question_digest"],"reasons":reasons,
        "thresholds_calibrated":false,"human_response":report["human_response"]});
    ensure!(
        detail.to_string().len() <= 64 * 1024,
        "classification history detail exceeds 64 KiB"
    );
    let hash = blake3::hash(&serde_json::to_vec(
        &json!({"model":botica_core::review::classifier::MODEL,"thresholds":report["thresholds"]}),
    )?)
    .to_hex()
    .to_string();
    let run = RunLine {
        v: 1,
        run_id: history::run_id(started),
        started_at: history::unix_seconds(started),
        commit: Some(head.into()),
        head: Some(head.into()),
        branch: None,
        botica: env!("CARGO_PKG_VERSION").into(),
        rules_fingerprint: hash,
        key_id: None,
        mode: origin.into(),
        origin: origin.into(),
        rules: BTreeMap::new(),
        detail: Some(detail),
    };
    history::append(root, &run, &[])
}
