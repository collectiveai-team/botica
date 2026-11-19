use super::{classify, evaluate};
use crate::output::println;
use anyhow::{Context, Result};
use clap::Args;
use serde_json::{json, Value};
use std::process::ExitCode;

#[derive(Args)]
pub(super) struct AutomationArgs {
    #[command(flatten)]
    classification: classify::ClassifyArgs,
}

pub(super) fn run(args: &AutomationArgs) -> Result<ExitCode> {
    let (assessment, mut report) = classify::collect(&args.classification)?;
    let enabled = assessment
        .review
        .as_ref()
        .is_some_and(|review| review.classifier.mode == botica_config::ClassifierMode::Enforce);
    let evaluation = trusted_evaluation(&assessment, &report)
        .unwrap_or_else(|_| json!({"ready_for_activation":false,"status":"unavailable"}));
    let protection = args
        .classification
        .repository
        .as_deref()
        .zip(args.classification.github_pr)
        .is_some_and(|(repo, pr)| {
            super::github::automation_policy(repo, pr, &report["inputs"]).unwrap_or(false)
        });
    report["checks"]["automation_policy_verified"] = json!(protection);
    let decision = botica_core::review::automation::authorize(
        enabled,
        &report["inputs"],
        &report,
        &evaluation,
    );
    println!("{}", serde_json::to_string_pretty(&decision)?);
    Ok(ExitCode::SUCCESS)
}

fn trusted_evaluation(assessment: &super::tier::Assessment, report: &Value) -> Result<Value> {
    let bytes = assessment
        .git
        .blob("bench/review-classifier/cases.jsonl")?
        .context("destination has no evaluated set")?;
    let cases = evaluate::parse_cases(std::str::from_utf8(&bytes)?)?;
    let thresholds = serde_json::from_value(report["thresholds"].clone())?;
    botica_core::review::evaluation::evaluate(&cases, &thresholds)
}
