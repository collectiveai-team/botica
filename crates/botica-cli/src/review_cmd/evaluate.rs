use super::evidence;
use crate::output::println;
use anyhow::{ensure, Context, Result};
use botica_config::Tier;
use botica_core::review::classifier::Thresholds;
use botica_core::review::evaluation::{evaluate, EvaluationCase};
use botica_core::review::reasons::ReasonCode;
use clap::Args;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Args)]
pub(super) struct EvaluateArgs {
    #[arg(long)]
    dataset: PathBuf,
    /// Recorded handoff JSON files. Exactly one observation per evaluated head.
    #[arg(long)]
    observation: Vec<PathBuf>,
    #[arg(long, default_value = "0.1")]
    maximum_question: f64,
    #[arg(long, default_value = "0.9")]
    minimum_contained_team: f64,
    #[arg(long, default_value = "0.7")]
    minimum_confidence: f64,
}

pub(super) fn read_cases(path: &Path) -> Result<Vec<EvaluationCase>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "evaluation set exceeds 8 MiB"
    );
    let text = std::str::from_utf8(&bytes)?;
    parse_cases(text)
}

pub(super) fn parse_cases(text: &str) -> Result<Vec<EvaluationCase>> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            serde_json::from_str(line)
                .map_err(|_| anyhow::anyhow!("invalid evaluation case at line {}", index + 1))
        })
        .collect()
}

fn apply_observation(cases: &mut [EvaluationCase], report: &Value) -> Result<()> {
    let head = report["inputs"]["head"]
        .as_str()
        .context("observation has no head")?;
    let case = cases
        .iter_mut()
        .find(|case| case.head == head)
        .context("observation is outside this set")?;
    ensure!(
        report["inputs"]["base"] == case.base,
        "evaluation observation has a different base"
    );
    let reasons = report["reasons"]
        .as_array()
        .context("observation has no reason list")?;
    ensure!(
        report["schema_version"] == 1
            && report["mode"] == "shadow"
            && report["merge_performed"] == false
            && report["inputs"].is_object()
            && report["review_summary"]["inputs"] == report["inputs"]
            && report["checks"]["inputs"] == report["inputs"],
        "evaluation observation has incomplete or mismatched evidence"
    );
    case.floor = report["deterministic_tier"]
        .as_str()
        .or_else(|| report["effective_tier"].as_str())
        .map(str::to_owned);
    case.critical = report["human_required"].as_bool();
    case.inputs_verified = report["checks"]["producer_verified"] == true
        && report["review_summary"]["producer_verified"] == true
        && reasons.iter().all(
            |reason| match reason["code"].as_str().and_then(ReasonCode::parse) {
                Some(code) if code.from_model() => reason["source"] == "Jev",
                Some(ReasonCode::CriticalArea) => report["human_required"] == true,
                Some(ReasonCode::RiskNotLow) => report["effective_tier"]
                    .as_str()
                    .and_then(Tier::parse)
                    .is_some_and(|tier| tier > Tier::Low),
                _ => false,
            },
        );
    case.checks_passed = Some(
        report["checks"]["ci"] == "PASS"
            && report["checks"]["lint"] == "PASS"
            && (!reasons
                .iter()
                .any(|reason| reason["code"] == json!(ReasonCode::CheckEvidence))),
    );
    case.reviews_passed = Some(
        report["review_summary"]["status"] == "complete"
            && report["review_summary"]["verdict"] == "PASS",
    );
    case.context_complete = Some(
        report["context_verified"] == true
            && !reasons.iter().any(|reason| {
                reason["code"]
                    .as_str()
                    .and_then(ReasonCode::parse)
                    .is_some_and(ReasonCode::context_incomplete)
            }),
    );
    case.probabilities = (!report["probabilities"].is_null())
        .then(|| serde_json::from_value(report["probabilities"].clone()))
        .transpose()?;
    case.question_digest = report["question_digest"].as_str().map(str::to_owned);
    Ok(())
}

pub(super) fn run(args: &EvaluateArgs) -> Result<ExitCode> {
    let mut cases = read_cases(&args.dataset)?;
    let mut heads = std::collections::BTreeSet::new();
    for path in &args.observation {
        let report = evidence::read_json(path)?;
        ensure!(
            heads.insert(report["inputs"]["head"].clone().to_string()),
            "duplicate evaluation observation"
        );
        apply_observation(&mut cases, &report)?;
    }
    let thresholds = Thresholds {
        maximum_question: args.maximum_question,
        minimum_contained_team: args.minimum_contained_team,
        minimum_confidence: args.minimum_confidence,
    };
    let mut report = evaluate(&cases, &thresholds)?;
    report["dataset_digest"] = serde_json::json!(blake3::hash(&serde_json::to_vec(&cases)?)
        .to_hex()
        .to_string());
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(ExitCode::SUCCESS)
}
