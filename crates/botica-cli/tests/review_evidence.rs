//! Freshness and verdict validation through the public review and lint commands.

mod common;
#[path = "review_evidence/history.rs"]
mod history;
use history::assert_readable_history;
#[cfg(unix)]
#[path = "review_evidence/github.rs"]
mod github;
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
    configured_fixture(name, "[review]\nroles=['spec','refute']\n")
}

fn configured_fixture(name: &str, config: &str) -> (PathBuf, Value) {
    let dir = common::project_in("review-evidence", name, Some(config));
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
    common::write(
        &dir,
        ".tmp/context.json",
        r#"{"title":"Return 42","description":"Implement the decided answer","acceptance_criteria":["The answer is 42"]}"#,
    );
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
            "--context",
            ".tmp/context.json",
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
            "--review-context",
            ".tmp/context.json",
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
    assert_readable_history(&dir, 2);
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
        &[
            "lint",
            "--reviewed",
            ".tmp/results",
            "--diff",
            "main",
            "--review-context",
            ".tmp/context.json",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(common::stderr(&output).contains("stale review"));
    report["inputs"] = plan["inputs"].clone();
    report["verdict"] = json!("INCONCLUSIVE");
    common::write(&dir, ".tmp/results/spec.json", &report.to_string());
    let output = common::command(
        &dir,
        &[
            "lint",
            "--reviewed",
            ".tmp/results",
            "--diff",
            "main",
            "--review-context",
            ".tmp/context.json",
        ],
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
    assert_readable_history(&dir, 1);
    let line = std::fs::read_to_string(dir.join(".botica/history/runs.jsonl")).unwrap();
    let recorded: Value = serde_json::from_str(line.lines().last().unwrap()).unwrap();
    assert_eq!(recorded["rules"], json!({}));
    assert!(recorded["detail"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason["code"] == "check-evidence"));
}

#[test]
fn refutation_quotes_are_checked_against_the_pinned_source_before_dismissing_a_p1() {
    let (dir, plan) = fixture("refutation-proof");
    let finding = json!({"id":"spec-1","severity":"P1","path":"src/answer.rs","line":1,
        "title":"Wrong public answer","evidence":"The function returns 42","reproduced":"not-tried",
        "how_to_verify":["Inspect answer"],"fix":"Return the intended value"});
    for role in ["spec", "refute"] {
        let mut report = evidence_report(&dir, role);
        report["findings"] = json!([finding]);
        if role == "refute" {
            report["findings"][0]["refutation"] = json!("unreachable");
            report["findings"][0]["refutation_evidence"] =
                json!("This new function has no caller in the fixture.");
            report["findings"][0]["refutation_references"] = json!([{"path":"src/answer.rs","revision":plan["inputs"]["head"],
                "line":1,"quote":"an invented quote"}]);
        }
        common::write(
            &dir,
            &format!(".tmp/results/{role}.json"),
            &report.to_string(),
        );
    }
    let args = [
        "lint",
        "--reviewed",
        ".tmp/results",
        "--diff",
        "main",
        "--review-context",
        ".tmp/context.json",
    ];
    let output = common::command(&dir, &args);
    assert_eq!(output.status.code(), Some(2));
    assert!(common::stderr(&output).contains("refutation quote"));
    let mut report = evidence_report(&dir, "refute");
    report["findings"][0]["refutation_references"][0]["quote"] = json!("pub fn answer()");
    common::write(&dir, ".tmp/results/refute.json", &report.to_string());
    let output = common::command(&dir, &args);
    assert_eq!(output.status.code(), Some(1), "{}", common::stderr(&output));
}

fn evidence_report(dir: &Path, role: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(dir.join(format!(".tmp/results/{role}.json"))).unwrap(),
    )
    .unwrap()
}

#[cfg(unix)]
#[test]
fn feedback_records_historical_human_decisions_without_authorizing_the_current_head() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, plan) = fixture("feedback");
    let report = json!({"schema_version":1,"mode":"shadow","inputs":plan["inputs"],
        "recommendation":"blocked","human_required":false,"automation_eligible":false,
        "merge_performed":false,"probabilities":null,"thresholds":{}});
    common::write(&dir, ".tmp/handoff.json", &report.to_string());
    common::write(
        &dir,
        ".tmp/pull.json",
        &json!({"state":"closed","merged":true,"base":{"sha":"e".repeat(40)},
        "head":{"sha":"d".repeat(40)}})
        .to_string(),
    );
    common::write(&dir,".tmp/reviews.json",&json!([
        {"id":1,"user":{"type":"User","login":"owner"},"commit_id":plan["inputs"]["head"],"state":"APPROVED","html_url":"https://github.com/owner/repo/pull/42#review-1"},
        {"id":2,"user":{"type":"Bot","login":"bot"},"commit_id":plan["inputs"]["head"],"state":"APPROVED"},
        {"id":3,"user":{"type":"User","login":"other"},"commit_id":"f".repeat(40),"state":"APPROVED"},
        {"id":4,"user":{"type":"User","login":"owner"},"commit_id":plan["inputs"]["head"],"state":"DISMISSED","html_url":"https://github.com/owner/repo/pull/42#review-4"}
    ]).to_string());
    common::write(
        &dir,
        ".tmp/bin/gh",
        r#"#!/bin/sh
case "$2" in
  *reviews*) cat "$FAKE_GH_ROOT/.tmp/reviews.json" ;;
  *) cat "$FAKE_GH_ROOT/.tmp/pull.json" ;;
esac
"#,
    );
    std::fs::set_permissions(
        dir.join(".tmp/bin/gh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args([
            "review",
            "feedback",
            "--handoff",
            ".tmp/handoff.json",
            "--repository",
            "owner/repo",
            "--github-pr",
            "42",
        ])
        .current_dir(&dir)
        .env("FAKE_GH_ROOT", &dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                dir.join(".tmp/bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["reviews"].as_array().unwrap().len(), 1);
    assert_eq!(response["reviews"][0]["reviewer"], "owner");
    assert_eq!(response["reviews"][0]["state"], "DISMISSED");
    assert_eq!(response["commit"], plan["inputs"]["head"]);
    assert_eq!(response["authorization"], false);
    assert_eq!(response["handoff_producer_verified"], false);
    let history = std::fs::read_to_string(dir.join(".botica/history/runs.jsonl")).unwrap();
    let recorded: Value = serde_json::from_str(history.lines().last().unwrap()).unwrap();
    assert_eq!(recorded["detail"]["human_response"], response);
    assert_eq!(recorded["detail"]["merge_performed"], false);
    assert_eq!(recorded["head"], plan["inputs"]["head"]);
    assert_eq!(recorded["mode"], "feedback");

    let observation = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args([
            "review",
            "feedback",
            "--head",
            plan["inputs"]["head"].as_str().unwrap(),
            "--repository",
            "owner/repo",
            "--github-pr",
            "42",
        ])
        .current_dir(&dir)
        .env("FAKE_GH_ROOT", &dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                dir.join(".tmp/bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .unwrap();
    assert!(
        observation.status.success(),
        "{}",
        common::stderr(&observation)
    );
    let response: Value = serde_json::from_slice(&observation.stdout).unwrap();
    assert_eq!(response["authorization"], false);
    let history = std::fs::read_to_string(dir.join(".botica/history/runs.jsonl")).unwrap();
    let recorded: Value = serde_json::from_str(history.lines().last().unwrap()).unwrap();
    assert_eq!(recorded["detail"]["observation_only"], true);
    assert!(recorded["detail"]["recommendation"].is_null());
    assert_eq!(recorded["head"], plan["inputs"]["head"]);
    assert_readable_history(&dir, 2);
}
