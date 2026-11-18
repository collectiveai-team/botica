use super::{consumers, evidence, jev::Jev};
use crate::output::println;
use anyhow::Result;
use botica_core::review::classifier::{recommend, Classifier, Thresholds};
use clap::Args;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::ExitCode;

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
        "context-limit" => ("Inspect the exact context byte count; no files were silently removed.", "Provide a review boundary that fits the bounded context."),
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
    let mut text = format!("<!-- botica:review-handoff -->\n# Review recommendation: {}\n\nHuman review required: {}. Shadow mode, no merge performed.\n\n",
        report["recommendation"].as_str().unwrap_or("blocked"), report["human_required"]);
    text.push_str(&format!("Inputs:\n\n    {}\n\n", report["inputs"]));
    text.push_str(&format!(
        "Decision requested: {}\n\n",
        report["decision_requested"]
            .as_str()
            .unwrap_or("resolve-evidence-blockers")
    ));
    text.push_str("## Controls reviewed\n\n");
    for role in report["review_summary"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        text.push_str(&format!(
            "    {}\n",
            json!({"role":role["role"],"verdict":role["verdict"]})
        ));
    }
    text.push_str("\n## Not reviewed\n\n");
    for item in report["not_reviewed"].as_array().into_iter().flatten() {
        text.push_str(&format!("    {item}\n"));
    }
    text.push('\n');
    for reason in report["reasons"].as_array().into_iter().flatten() {
        text.push_str(&format!(
            "## {}\n\nSource: {}\n\nEvidence:\n\n    {}\n\nVerify: {}\n\nResolve: {}\n\n",
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
    let mut results = evidence::collect(&assessment, &plan, &args.results)
        .unwrap_or_else(|_| json!({"status":"incomplete","verdict":"INCONCLUSIVE","inputs":null}));
    let provenance = args.github_pr.and_then(|pr| {
        super::github::verify_roles(
            args.repository.as_deref().unwrap_or(""),
            pr,
            &assessment,
            &plan,
            &results,
        )
        .ok()
    });
    results["producer_verified"] = json!(provenance.is_some());
    results["provenance"] = provenance.unwrap_or(Value::Null);
    let checks = if let Some(pr) = args.github_pr {
        super::github::checks(args.repository.as_deref().unwrap_or(""), pr, &plan)
            .unwrap_or_else(|_| json!({"producer_verified":false,"status":"unavailable"}))
    } else {
        let mut checks = args
            .checks
            .as_deref()
            .and_then(|path| evidence::read_json(path).ok())
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({}));
        checks["producer_verified"] = json!(false);
        checks
    };
    let context = plan["untrusted_context"].clone();
    let context_verified = args.github_pr.is_some_and(|pr| {
        super::github::context_matches(args.repository.as_deref().unwrap_or(""), pr, &plan)
            .unwrap_or(false)
    });
    let diff = super::change_context::collect(&assessment,&plan,96 * 1024)
        .unwrap_or_else(|_| json!({"status":"unavailable","reason":"bounded change context could not be collected"}));
    let state = json!({"context":context,"plan":plan,"reviews":results,"checks":checks,
        "context_verified":context_verified,"change_context_complete":diff.is_array(),"diff":diff});
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
    if context_verified
        && args.github_pr.is_some_and(|pr| {
            !super::github::context_matches(args.repository.as_deref().unwrap_or(""), pr, &plan)
                .unwrap_or(false)
        })
    {
        report["automation_eligible"] = json!(false);
        if report["human_required"] != true {
            report["recommendation"] = json!("blocked");
        }
        report["reasons"].as_array_mut().unwrap().push(json!({"code":"context-changed",
            "source":"gate","evidence":"The GitHub acceptance context changed during classification."}));
    }
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
    report["not_reviewed"] = json!(not_reviewed(&plan, &results, &args.results));
    report["decision_requested"] = json!(if report["human_required"] == true {
        "critical-owner-review"
    } else if report["automation_eligible"] == true {
        "none-shadow-only"
    } else {
        "resolve-evidence-blockers"
    });
    report["review_summary"] = results;
    report["checks"] = checks;
    if report["automation_eligible"] == true && !fresh_evidence(args, &assessment, &plan, &report) {
        report["automation_eligible"] = json!(false);
        report["recommendation"] = json!("blocked");
        report["reasons"].as_array_mut().unwrap().push(json!({"code":"evidence-changed",
            "source":"gate","evidence":"Role, check or acceptance evidence changed during classification."}));
        report["decision_requested"] = json!("resolve-evidence-blockers");
    }
    handoff(&mut report);
    Ok((assessment, report))
}

fn fresh_evidence(
    args: &ClassifyArgs,
    assessment: &super::tier::Assessment,
    plan: &Value,
    report: &Value,
) -> bool {
    let Some((repository, pr)) = args.repository.as_deref().zip(args.github_pr) else {
        return false;
    };
    let verified = || -> Result<bool> {
        let roles = super::github::verify_roles(
            repository,
            pr,
            assessment,
            plan,
            &report["review_summary"],
        )?;
        let checks = super::github::checks(repository, pr, plan)?;
        Ok(roles == report["review_summary"]["provenance"]
            && checks == report["checks"]
            && super::github::context_matches(repository, pr, plan)?)
    };
    verified().unwrap_or(false)
}

pub(super) fn run(args: &ClassifyArgs) -> Result<ExitCode> {
    let (assessment, report) = collect(args)?;
    write_output(&args.output, &report)?;
    super::classification_history::record(&assessment, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(ExitCode::SUCCESS)
}
