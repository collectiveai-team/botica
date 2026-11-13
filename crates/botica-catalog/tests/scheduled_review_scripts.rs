//! The `run:` bodies of the jobs `plan`, `mutants` and `report` of
//! `.github/workflows/scheduled-review.yml`, run with bash on repositories and reports of shards
//! that a test writes, as the runner runs them. `jq` and `git` are needed: a test skips itself
//! without `jq`, unless `BOTICA_REQUIRE_WORKFLOW_LINTERS` is set (CI).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use common::workflows::{repository_workflow, steps};
use serde_json::json;
use serde_yaml::Value;

fn job(name: &str) -> Value {
    repository_workflow("scheduled-review.yml")["jobs"][name].clone()
}

fn report_job() -> Value {
    job("report")
}

fn step_named(job: &Value, name: &str) -> String {
    steps(job)
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("no step named {name}"))
        .to_owned()
}

fn step_with_id(job: &Value, id: &str) -> String {
    steps(job)
        .iter()
        .find(|s| s["id"].as_str() == Some(id))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("no step with id {id}"))
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

fn git(root: &Path, args: &[&str], date: &str) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .output()
        .expect("git runs");
    assert!(output.status.success(), "{args:?}: {}", stderr(&output));
}

/// A repository whose commits are `(days ago, file, text)`, oldest first.
fn history(name: &str, commits: &[(u32, &str, &str)]) -> PathBuf {
    let root = scratch(name);
    git(
        &root,
        &["init", "--quiet", "-b", "main"],
        "2020-01-01T00:00:00Z",
    );
    for (days, file, text) in commits {
        fs::write(root.join(file), text).expect("the file");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after 1970")
            .as_secs();
        let when = format!("{} +0000", now - u64::from(*days) * 86_400);
        git(&root, &["add", "."], &when);
        git(&root, &["commit", "--quiet", "-m", "change"], &when);
    }
    root
}

fn plan(root: &Path) -> (Output, String) {
    let script = step_with_id(&job("plan"), "plan");
    let output = bash(root, &script, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let outputs = fs::read_to_string(root.join("output.txt")).expect("the outputs");
    (output, outputs)
}

#[test]
fn the_plan_takes_the_rust_lines_that_changed_since_the_last_commit_a_week_old() {
    if !has_jq() {
        return;
    }
    let root = history(
        "scheduled-review-plan-rust",
        &[
            (30, "lib.rs", "fn old() {}\n"),
            (20, "notes.md", "old\n"),
            (2, "lib.rs", "fn old() {}\nfn new() {}\n"),
        ],
    );

    let (_, outputs) = plan(&root);

    assert_eq!(outputs.trim(), "rust=true");
    let diff = fs::read_to_string(root.join("week.diff")).expect("the diff");
    assert!(diff.contains("+fn new() {}"), "{diff}");
    assert!(!diff.contains("+fn old() {}"), "{diff}");
}

#[test]
fn the_plan_says_so_and_ends_the_mutation_when_the_week_changed_no_rust() {
    if !has_jq() {
        return;
    }
    let root = history(
        "scheduled-review-plan-quiet",
        &[
            (30, "lib.rs", "fn old() {}\n"),
            (2, "notes.md", "a page of the docs\n"),
        ],
    );

    let (_, outputs) = plan(&root);

    assert_eq!(outputs.trim(), "rust=false");
    let summary = fs::read_to_string(root.join("summary.md")).expect("the summary");
    assert!(summary.contains("nothing to mutate"), "{summary}");
    assert_eq!(
        fs::metadata(root.join("week.diff")).map(|m| m.len()).ok(),
        Some(0)
    );
}

#[test]
fn the_plan_of_a_repository_younger_than_a_week_takes_every_rust_line() {
    if !has_jq() {
        return;
    }
    let root = history(
        "scheduled-review-plan-young",
        &[(1, "lib.rs", "fn first() {}\n")],
    );

    let (_, outputs) = plan(&root);

    assert_eq!(outputs.trim(), "rust=true");
    let diff = fs::read_to_string(root.join("week.diff")).expect("the diff");
    assert!(diff.contains("+fn first() {}"), "{diff}");
}

/// Runs the step of cargo-mutants of the job `mutants` in `shards/mutants-shard-0` of `root`, with a
/// `cargo` that exits with `status` and writes nothing, as `cargo mutants` does for an empty shard.
fn empty_shard(root: &Path, status: i32) -> Output {
    let bin = root.join("bin");
    fs::create_dir_all(&bin).expect("the bin folder");
    let cargo = bin.join("cargo");
    fs::write(&cargo, format!("#!/bin/sh\nexit {status}\n")).expect("the fake cargo");
    Command::new("chmod")
        .arg("+x")
        .arg(&cargo)
        .status()
        .expect("chmod runs");
    let workdir = root.join("shards/mutants-shard-0");
    fs::create_dir_all(&workdir).expect("the shard folder");
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    bash(
        &workdir,
        &step_named(&job("mutants"), "Test the mutants of the shard"),
        &[
            ("PATH", &path),
            ("RUNNER_TEMP", "/nonexistent"),
            ("SHARD", "0"),
            ("MUTANT_SHARDS", "4"),
        ],
    )
}

#[test]
fn a_shard_with_no_mutant_leaves_an_empty_report_that_the_check_counts() {
    if !has_jq() {
        return;
    }
    let root = scratch("scheduled-review-empty-shard");

    let run = empty_shard(&root, 0);

    assert!(run.status.success(), "{}", stderr(&run));
    let job = report_job();
    shard(&root, 1, &["CaughtMutant"], 1);
    let join = bash(
        &root,
        &step_named(&job, "Join the reports of the shards"),
        &[],
    );
    assert!(join.status.success(), "{}", stderr(&join));
    let output = fs::read_to_string(root.join("output.txt")).expect("the outputs");
    assert_eq!(output.lines().collect::<Vec<_>>(), ["found=2", "partial=0"]);
}

#[test]
fn a_shard_whose_cargo_mutants_failed_fails_the_step() {
    if !has_jq() {
        return;
    }
    let root = scratch("scheduled-review-failed-shard");

    let run = empty_shard(&root, 1);

    assert!(!run.status.success());
}

#[test]
fn a_run_with_no_report_scores_not_applicable() {
    if !has_jq() {
        return;
    }
    let job = report_job();
    let root = scratch("scheduled-review-no-report");

    let join = bash(
        &root,
        &step_named(&job, "Join the reports of the shards"),
        &[],
    );
    let score = bash(&root, &step_named(&job, "Score"), &[]);

    assert!(join.status.success(), "{}", stderr(&join));
    assert!(score.status.success(), "{}", stderr(&score));
    let output = fs::read_to_string(root.join("output.txt")).expect("the outputs");
    assert_eq!(output.lines().collect::<Vec<_>>(), ["found=0", "partial=0"]);
    let summary = fs::read_to_string(root.join("summary.md")).expect("the summary");
    assert!(summary.contains("| 0 | 0 | 0 | 0 | n/a |"), "{summary}");
}
