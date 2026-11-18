//! Freshness and verdict validation through the public review and lint commands.

mod common;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture(name: &str) -> (PathBuf, Value) {
    let dir = common::project_in(
        "review-evidence",
        name,
        Some("[review]\nroles=['spec','refute']\n"),
    );
    for role in ["spec", "refute"] {
        common::write(&dir, &format!(".github/review/{role}/REVIEW.md"),
            &format!("---\nrole: {role}\ncovered: {{}}\n---\n<!-- botica:covered -->\n<!-- /botica:covered -->\n"));
    }
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Review test"]);
    git(&dir, &["config", "user.email", "review@example.com"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Trusted policy"]);
    git(&dir, &["switch", "-c", "topic"]);
    common::write(&dir, "src/answer.rs", "pub fn answer() -> u8 { 42 }\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Change source"]);
    let output = common::command(
        &dir,
        &[
            "review",
            "plan",
            "--diff",
            "main",
            "--integration-method",
            "merge",
            "--format",
            "json",
        ],
    );
    assert!(output.status.success(), "{}", common::stderr(&output));
    let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
    for role in ["spec", "refute"] {
        let report = json!({"role":role,"head":plan["inputs"]["head"],"base":plan["inputs"]["base"],"inputs":plan["inputs"],
            "verdict":"PASS","findings":[],"checked":["src/answer.rs"],"not_checked":[],"commands":[]});
        common::write(
            &dir,
            &format!(".tmp/results/{role}.json"),
            &report.to_string(),
        );
    }
    (dir, plan)
}

#[test]
fn lint_ingests_current_reviews_and_records_each_role_without_running_commands() {
    let (dir, _) = fixture("record");
    let output = common::command(
        &dir,
        &[
            "lint",
            "--reviewed",
            ".tmp/results",
            "--diff",
            "main",
            "--record",
        ],
    );
    assert!(output.status.success(), "{}", common::stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verdict"], "PASS");
    let runs = std::fs::read_to_string(dir.join(".botica/history/runs.jsonl")).unwrap();
    let lines: Vec<Value> = runs
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line["mode"] == "reviewed"));
    assert!(lines.iter().any(|line| line["origin"] == "review:spec"));
}

#[test]
fn missing_and_stale_role_results_fail_the_public_gate() {
    let (dir, plan) = fixture("stale");
    let mut report: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(".tmp/results/spec.json")).unwrap())
            .unwrap();
    report["inputs"]["head"] = json!("f".repeat(40));
    common::write(&dir, ".tmp/results/spec.json", &report.to_string());
    let output = common::command(
        &dir,
        &["lint", "--reviewed", ".tmp/results", "--diff", "main"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(common::stderr(&output).contains("stale review"));
    report["inputs"] = plan["inputs"].clone();
    report["verdict"] = json!("INCONCLUSIVE");
    common::write(&dir, ".tmp/results/spec.json", &report.to_string());
    let output = common::command(
        &dir,
        &["lint", "--reviewed", ".tmp/results", "--diff", "main"],
    );
    assert_eq!(output.status.code(), Some(2));
    let validated: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(validated["verdict"], "INCONCLUSIVE");
}

#[test]
fn shadow_classification_writes_a_traceable_blocked_report_without_human_routing_or_model_access() {
    let (dir, plan) = fixture("shadow-blocked");
    common::write(
        &dir,
        ".tmp/context.json",
        r#"{"title":"Change source","issue":"Return 42"}"#,
    );
    common::write(
        &dir,
        ".tmp/checks.json",
        &json!({
            "inputs":plan["inputs"],"ci":"FAIL","lint":"PASS","mutation":"PASS"
        })
        .to_string(),
    );
    let output = common::command(
        &dir,
        &[
            "review",
            "classify",
            "--diff",
            "main",
            "--integration-method",
            "merge",
            "--results",
            ".tmp/results",
            "--checks",
            ".tmp/checks.json",
            "--context",
            ".tmp/context.json",
            "--output",
            ".tmp/handoff",
        ],
    );
    assert!(output.status.success(), "{}", common::stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["mode"], "shadow");
    assert_eq!(report["recommendation"], "blocked");
    assert_eq!(report["human_required"], false);
    assert_eq!(report["merge_performed"], false);
    assert_eq!(report["inputs"]["head"], plan["inputs"]["head"]);
    assert_ne!(
        report["inputs"]["context_digest"],
        plan["inputs"]["context_digest"]
    );
    assert!(report["probabilities"].is_null());
    let saved: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".tmp/handoff/handoff.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, report);
    assert!(report["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason["code"] == "check-evidence"
            && reason["verify"].is_string()
            && reason["resolve"].is_string()));
}
