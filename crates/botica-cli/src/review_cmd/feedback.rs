use super::{evidence, github, tier};
use crate::output::println;
use anyhow::{ensure, Result};
use clap::Args;
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Args)]
pub(super) struct FeedbackArgs {
    #[arg(long, required_unless_present = "head", conflicts_with = "head")]
    handoff: Option<PathBuf>,
    /// Observe reviews of this commit without claiming a classification was produced.
    #[arg(long, required_unless_present = "handoff")]
    head: Option<String>,
    #[arg(long)]
    repository: String,
    #[arg(long)]
    github_pr: u64,
}

pub(super) fn run(args: &FeedbackArgs) -> Result<ExitCode> {
    let mut report = if let Some(path) = &args.handoff {
        evidence::read_json(path)?
    } else {
        json!({"schema_version":1,"mode":"feedback","inputs":{"head":args.head},
            "merge_performed":false,"observation_only":true})
    };
    ensure!(
        report["schema_version"] == 1
            && (report["mode"] == "shadow" || report["mode"] == "feedback")
            && report["merge_performed"] == false,
        "feedback requires an observation or shadow handoff"
    );
    if report["checks"]["repository"].is_string() {
        ensure!(
            report["checks"]["repository"] == args.repository
                && report["checks"]["pr"] == args.github_pr,
            "handoff belongs to another GitHub PR"
        );
    }
    let head = report["inputs"]["head"]
        .as_str()
        .filter(|head| {
            matches!(head.len(), 40 | 64) && head.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .ok_or_else(|| anyhow::anyhow!("handoff has no full head object ID"))?;
    let reviews = github::reviews(&args.repository, args.github_pr, head)?;
    report["human_response"] = json!({"source":"github","repository":args.repository,
        "pr":args.github_pr,"commit":head,"reviews":reviews,
        "authorization":false,"handoff_producer_verified":false});
    let git = tier::git::Git::open("HEAD", "HEAD")?;
    super::classification_history::append(&git.root, &report, "feedback")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report["human_response"])?
    );
    Ok(ExitCode::SUCCESS)
}
