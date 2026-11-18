use super::{consumers, evidence, jev::Jev};
use anyhow::Result;
use botica_core::review::classifier::{recommend, Classifier, Thresholds};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args)]
pub(super) struct ClassifyArgs {
    #[command(flatten)]
    plan: consumers::ConsumerArgs,
    #[arg(long)]
    results: PathBuf,
    /// Check attestations with exactly the current plan inputs and CI/lint/mutation statuses.
    #[arg(long)]
    checks: PathBuf,
    #[arg(long, default_value = ".botica/review")]
    output: PathBuf,
}

fn reason_steps(code: &str) -> (&'static str, &'static str) {
    match code {
        "critical-area" => ("Inspect the matched critical paths and their owners in the risk evidence.", "Obtain human review from the critical area owner."),
        "incomplete-risk" => ("Run botica review tier for the pinned destination, head and integration method.", "Resolve missing history, unsupported inputs or integration conflicts."),
        "review-evidence" | "unresolved-finding" => ("Inspect role verdicts, findings and not_checked for this change.", "Resolve surviving findings and rerun missing or inconclusive roles."),
        "check-evidence" => ("Inspect CI, lint and mutation attestations and compare their full input binding.", "Rerun the missing or failed checks for the current inputs."),
        "decision" => ("Compare the located decision with the linked issue and description.", "Record the intended choice in the issue and verify its implementation."),
        "unverifiable" => ("Run the changed behavior in base and head and compare observable output.", "Add the missing direct verification evidence."),
        "impact-users" => ("Inspect the published CLI, rule or recipe surface and its consumers.", "Verify the changed public behavior in a disposable repository."),
        "risk-not-low" => ("Inspect both branch and integration risk evidence.", "Keep automation blocked under the low-risk automation policy."),
        "context-limit" => ("Inspect the exact context byte count; no files were silently removed.", "Provide a review boundary that fits the bounded context."),
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
    let mut text = format!("<!-- botica:review-handoff -->\n# Review recommendation: {}\n\nHuman review required: {}. Shadow mode, no merge performed.\n\n",
        report["recommendation"].as_str().unwrap_or("blocked"), report["human_required"]);
    text.push_str(&format!("Inputs: `{}`\n\n", report["inputs"]));
    for reason in report["reasons"].as_array().into_iter().flatten() {
        text.push_str(&format!(
            "## {}\n\nSource: {}\n\nEvidence: `{}`\n\nVerify: {}\n\nResolve: {}\n\n",
            reason["code"].as_str().unwrap_or("blocked"),
            reason["source"],
            reason,
            reason["verify"].as_str().unwrap_or(""),
            reason["resolve"].as_str().unwrap_or("")
        ));
    }
    std::fs::write(&markdown_path, text)?;
    Ok(())
}

pub(super) fn run(args: &ClassifyArgs) -> Result<ExitCode> {
    let (assessment, plan) = consumers::collect(&args.plan, false)?;
    let results = evidence::collect(&plan, &args.results)
        .unwrap_or_else(|_| json!({"status":"incomplete","verdict":"INCONCLUSIVE","inputs":null}));
    let checks = evidence::read_json(&args.checks).unwrap_or(Value::Null);
    let context = plan["untrusted_context"].clone();
    let diff = super::change_context::collect(&assessment,&plan,96 * 1024)
        .unwrap_or_else(|_| json!({"status":"unavailable","reason":"bounded change context could not be collected"}));
    let state = json!({"context":context,"plan":plan,"reviews":results,"checks":checks,
        "change_context_complete":diff.is_array(),"diff":diff});
    let adapter = Jev::configured();
    let thresholds = assessment
        .review
        .as_ref()
        .map(|review| Thresholds {
            maximum_question: review.classifier.question_max,
            minimum_contained_team: review.classifier.impact_min,
            minimum_confidence: review.classifier.min_confidence,
        })
        .unwrap_or_default();
    let mut report = recommend(&plan, &results, &checks, &state, &adapter, &thresholds)?;
    let mut localized = Vec::new();
    for reason in report["reasons"].as_array().into_iter().flatten() {
        let code = reason["code"].as_str().unwrap_or("");
        if !matches!(code, "decision" | "unverifiable") {
            continue;
        }
        let mut paths = std::collections::BTreeSet::new();
        for comparison in plan["risk"]["branch"]["comparisons"]
            .as_array()
            .into_iter()
            .flatten()
        {
            paths.extend(
                comparison["files"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str),
            );
        }
        for path in paths {
            let content = assessment.git.read_limited(
                &["show", &format!("{}:{path}", assessment.git.head)],
                16_000,
            );
            let Ok(content) = content else {
                localized.push(json!({"reason":code,"path":path,"status":"unavailable"}));
                continue;
            };
            let state =
                json!({"context":context,"path":path,"file":String::from_utf8_lossy(&content)});
            let probability = adapter.locate(&state, code).ok();
            localized.push(json!({"reason":code,"path":path,"probability":probability,"status":if probability.is_some() {"measured"} else {"unavailable"}}));
        }
    }
    report["localization"] = json!(localized);
    report["review_summary"] = results;
    report["checks"] = checks;
    handoff(&mut report);
    write_output(&args.output, &report)?;
    super::classification_history::record(&assessment, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(ExitCode::SUCCESS)
}
