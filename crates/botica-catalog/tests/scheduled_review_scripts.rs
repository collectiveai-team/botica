//! The `run:` bodies of the job `report` of `.github/workflows/scheduled-review.yml`, run with bash
//! on reports of shards that a test writes, as the runner runs them. `jq` is needed: a test skips
//! itself without it, unless `BOTICA_REQUIRE_WORKFLOW_LINTERS` is set (CI).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::workflows::{repository_workflow, steps};
use serde_json::json;
use serde_yaml::Value;

fn report_job() -> Value {
    repository_workflow("scheduled-review.yml")["jobs"]["report"].clone()
}

fn step_named(job: &Value, name: &str) -> String {
    steps(job)
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("no step named {name}"))
        .to_owned()
}

fn shard(root: &Path, number: usize, summaries: &[&str], listed: usize) {
    let dir = root.join(format!("shards/mutants-shard-{number}/mutants.out"));
    fs::create_dir_all(&dir).expect("the shard folder");
    let mut outcomes = vec![json!({"scenario": "Baseline", "summary": "Success"})];
    outcomes.extend(
        summaries
            .iter()
            .map(|s| json!({"scenario": {"Mutant": {"name": "x"}}, "summary": s})),
    );
    fs::write(
        dir.join("outcomes.json"),
        json!({"outcomes": outcomes}).to_string(),
    )
    .expect("the outcomes");
    fs::write(
        dir.join("mutants.json"),
        format!("[{}]", vec!["{}"; listed].join(",")),
    )
    .expect("the list");
}

fn required() -> bool {
    std::env::var_os("BOTICA_REQUIRE_WORKFLOW_LINTERS").is_some()
}

fn has_jq() -> bool {
    let found = Command::new("jq").arg("--version").output().is_ok();
    assert!(
        found || !required(),
        "BOTICA_REQUIRE_WORKFLOW_LINTERS is set but `jq` is not installed"
    );
    found
}

fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the scratch folder");
    root
}

/// Runs `script` with bash as the runner does, in `root`, with the files of the step summary and of
/// the outputs of the step in `root`.
fn bash(root: &Path, script: &str, env: &[(&str, &str)]) -> Output {
    Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .current_dir(root)
        .env("GITHUB_OUTPUT", root.join("output.txt"))
        .env("GITHUB_STEP_SUMMARY", root.join("summary.md"))
        .envs(env.iter().copied())
        .output()
        .expect("bash runs")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_join_and_the_score_count_what_the_shards_tested() {
    if !has_jq() {
        return;
    }
    let job = report_job();
    let root = scratch("scheduled-review-score");
    shard(
        &root,
        0,
        &["CaughtMutant", "CaughtMutant", "MissedMutant"],
        3,
    );
    shard(&root, 1, &["CaughtMutant", "Timeout", "Unviable"], 3);

    let join = bash(
        &root,
        &step_named(&job, "Join the reports of the shards"),
        &[],
    );
    assert!(join.status.success(), "{}", stderr(&join));
    let score = bash(&root, &step_named(&job, "Score"), &[]);
    assert!(score.status.success(), "{}", stderr(&score));

    let output = fs::read_to_string(root.join("output.txt")).expect("the outputs");
    assert_eq!(output.lines().collect::<Vec<_>>(), ["found=2", "partial=0"]);
    let joined: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("mutants-all.json")).expect("joined"))
            .expect("JSON");
    assert_eq!(joined["outcomes"].as_array().map(Vec::len), Some(8));
    let summary = fs::read_to_string(root.join("summary.md")).expect("the summary");
    assert!(summary.contains("| 3 | 1 | 1 | 1 | 75% |"), "{summary}");
}

#[test]
fn a_shard_that_stopped_early_or_left_no_report_fails_the_check() {
    if !has_jq() {
        return;
    }
    let job = report_job();
    let root = scratch("scheduled-review-partial");
    shard(&root, 0, &["CaughtMutant"], 1);
    shard(&root, 1, &["CaughtMutant"], 5);

    let join = bash(
        &root,
        &step_named(&job, "Join the reports of the shards"),
        &[],
    );

    assert!(join.status.success(), "{}", stderr(&join));
    let output = fs::read_to_string(root.join("output.txt")).expect("the outputs");
    assert_eq!(output.lines().collect::<Vec<_>>(), ["found=2", "partial=1"]);
    let check = step_named(&job, "Check that every shard tested all of its mutants");
    let counts = |found, partial| {
        bash(
            &root,
            &check,
            &[
                ("MUTANT_SHARDS", "2"),
                ("FOUND", found),
                ("PARTIAL", partial),
            ],
        )
    };
    assert!(!counts("2", "1").status.success(), "a partial shard");
    assert!(!counts("1", "0").status.success(), "a missing report");
    assert!(counts("2", "0").status.success());
}
