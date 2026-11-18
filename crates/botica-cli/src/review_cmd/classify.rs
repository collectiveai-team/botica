use super::{consumers, jev::Jev};
use crate::output::println;
use anyhow::Result;
use botica_core::review::classifier::{handoff_markdown, recommend_current, Evidence};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

#[path = "classify_steps.rs"]
mod steps;

#[derive(Args)]
pub(super) struct ClassifyArgs {
    #[command(flatten)]
    pub(super) plan: consumers::ConsumerArgs,
    #[arg(long)]
    results: PathBuf,
    /// Check attestations with exactly the current plan inputs and CI/lint/mutation statuses.
    #[arg(long, conflicts_with = "github_pr")]
    checks: Option<PathBuf>,
    /// Read check conclusions from GitHub instead of accepting a local attestation.
    #[arg(long, requires = "repository")]
    pub(super) github_pr: Option<u64>,
    #[arg(long, requires = "github_pr")]
    pub(super) repository: Option<String>,
    #[arg(long, default_value = ".botica/review")]
    output: PathBuf,
}

fn reason_steps(code: &str) -> (&'static str, &'static str) {
    match code {
        "critical-area" => ("Inspect the matched critical paths and their owners in the risk evidence.", "Obtain human review from the critical area owner."),
        "incomplete-risk" => ("Run botica review tier for the pinned destination, head and integration method.", "Resolve missing history, unsupported inputs or integration conflicts."),
        "review-evidence" | "unresolved-finding" => ("Inspect role verdicts, findings and not_checked for this change.", "Resolve surviving findings and rerun missing or inconclusive roles."),
        "check-evidence" => ("Inspect CI, lint and mutation attestations and compare their full input binding.", "Rerun the missing or failed checks for the current inputs."),
        "evidence-changed" => ("Compare the latest role workflow and integration check IDs with the recorded snapshot.", "Wait for current checks and reviews, then rerun classification."),
        "decision" => ("Compare the located decision with the linked issue and description.", "Record the intended choice in the issue and verify its implementation."),
        "unverifiable" => ("Run the changed behavior in base and head and compare observable output.", "Add the missing direct verification evidence."),
        "impact-users" => ("Inspect the published CLI, rule or recipe surface and its consumers.", "Verify the changed public behavior in a disposable repository."),
        "risk-not-low" => ("Inspect both branch and integration risk evidence.", "Keep automation blocked under the low-risk automation policy."),
        "context-limit" => ("Inspect the model's reported input-token count against the 32,000-token acceptance budget.", "Use a smaller review boundary. The rejected response supplies no probabilities."),
        "context-byte-limit" => ("Inspect the local serialized-request byte limit, separately from the model's token budget.", "Use a smaller review boundary without dropping required evidence."),
        "missing-acceptance-context" | "context-unavailable" | "context-changed" => ("Compare the current PR description and linked issue bodies with the review context.", "Refresh context and rerun the affected roles and classification."),
        _ => ("Inspect model availability and the pinned model version; no service error body was retained.", "Retry the configured service. A technical failure does not require human review."),
    }
}

fn handoff(report: &mut Value) {
    if let Some(reasons) = report["reasons"].as_array_mut() {
        for reason in reasons {
            let (verify, resolve) = reason_steps(reason["code"].as_str().unwrap_or(""));
            reason["verify"] = json!(verify);
            reason["resolve"] = json!(resolve);
        }
    }
}

fn write_output(directory: &std::path::Path, report: &Value) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    anyhow::ensure!(
        !std::fs::symlink_metadata(directory)?
            .file_type()
            .is_symlink(),
        "handoff directory is a symbolic link"
    );
    let json_path = directory.join("handoff.json");
    let markdown_path = directory.join("handoff.md");
    for path in [&json_path, &markdown_path] {
        if let Ok(metadata) = std::fs::symlink_metadata(path) {
            anyhow::ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "handoff target is not a regular file"
            );
        }
    }
    std::fs::write(&json_path, serde_json::to_vec_pretty(report)?)?;
    std::fs::write(&markdown_path, handoff_markdown(report))?;
    Ok(())
}

fn not_reviewed(plan: &Value, results: &Value, directory: &std::path::Path) -> Vec<Value> {
    let mut gaps = Vec::new();
    for role in plan["roles"].as_array().into_iter().flatten() {
        let name = role["role"].as_str().unwrap_or("");
        if role["selection"]["selected"] != true {
            gaps.push(json!({"role":name,"reason":"not selected by destination policy","selection":role["selection"]}));
        } else if !directory.join(format!("{name}.json")).is_file()
            || results["status"] != "complete"
        {
            gaps.push(json!({"role":name,"reason":"valid complete result unavailable"}));
        }
    }
    for role in results["roles"].as_array().into_iter().flatten() {
        for item in role["not_checked"].as_array().into_iter().flatten() {
            gaps.push(json!({"role":role["role"],"reason":item}));
        }
    }
    gaps
}

pub(super) fn collect(args: &ClassifyArgs) -> Result<(super::tier::Assessment, Value)> {
    let (assessment, plan) = consumers::collect(&args.plan, false)?;
    let results = steps::roles(args, &assessment, &plan);
    let checks = steps::checks(args, &plan);
    let context_verified = steps::context_current(args, &plan);
    let diff = super::change_context::classification(&assessment,&plan,256 * 1024)
        .unwrap_or_else(|_| json!({"status":"unavailable","reason":"bounded change context could not be collected"}));
    let state = botica_core::review::state::build(&plan, &results, &checks, diff, context_verified);
    let adapter = Jev::configured();
    let thresholds = steps::thresholds(&assessment);
    let current = CurrentEvidence {
        args,
        assessment: &assessment,
        plan: &plan,
        results: &results,
        checks: &checks,
    };
    let mut report = recommend_current(
        &plan,
        &results,
        &checks,
        &state,
        &adapter,
        &thresholds,
        &current,
    )?;
    steps::invalidate_changed_context(&mut report, args, &plan, context_verified);
    let localized = steps::localization(&assessment, &plan, &adapter, &report);
    let gaps = not_reviewed(&plan, &results, &args.results);
    let report = steps::finish(report, localized, gaps, results, checks);
    Ok((assessment, report))
}

struct CurrentEvidence<'a> {
    args: &'a ClassifyArgs,
    assessment: &'a super::tier::Assessment,
    plan: &'a Value,
    results: &'a Value,
    checks: &'a Value,
}

impl Evidence for CurrentEvidence<'_> {
    fn is_current(&self) -> Result<bool> {
        let Some((repository, pr)) = self.args.repository.as_deref().zip(self.args.github_pr)
        else {
            return Ok(false);
        };
        let roles =
            super::github::verify_roles(repository, pr, self.assessment, self.plan, self.results)?;
        let checks = super::github::checks(repository, pr, self.plan)?;
        Ok(roles == self.results["provenance"]
            && checks == *self.checks
            && super::github::context_matches(repository, pr, self.plan)?)
    }
}

pub(super) fn run(args: &ClassifyArgs) -> Result<ExitCode> {
    let (assessment, report) = collect(args)?;
    write_output(&args.output, &report)?;
    super::classification_history::record(&assessment, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(ExitCode::SUCCESS)
}
