//! The job `history-report` of `.github/workflows/scheduled-review.yml`: the one job of the
//! workflow that writes, and what it reads. A `schedule` runs on the default branch only, so these
//! tests read the file; `scheduled_review_report.rs` runs the steps.

mod common;

use common::workflows::{repository_workflow, steps};
use serde_yaml::Value;

fn scheduled() -> Value {
    repository_workflow("scheduled-review.yml")
}

fn using<'a>(job: &'a Value, prefix: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|s| s["uses"].as_str().is_some_and(|u| u.starts_with(prefix)))
        .unwrap_or_else(|| panic!("no step uses {prefix}"))
}

fn step_run<'a>(job: &'a Value, name: &str) -> &'a str {
    steps(job)
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("no step named {name}"))
}

#[test]
fn the_report_job_alone_writes_issues_with_the_token_of_the_run() {
    let workflow = scheduled();
    let jobs = workflow["jobs"].as_mapping().expect("the jobs");

    let writers: Vec<_> = jobs
        .iter()
        .filter(|(_, job)| !job["permissions"].is_null())
        .map(|(name, _)| name.as_str().expect("a name"))
        .collect();
    assert_eq!(writers, ["history-report"]);
    let job = &workflow["jobs"]["history-report"];
    let permissions = job["permissions"].as_mapping().expect("permissions");
    assert_eq!(permissions.len(), 3, "{permissions:?}");
    assert_eq!(permissions["contents"].as_str(), Some("read"));
    assert_eq!(permissions["actions"].as_str(), Some("read"));
    assert_eq!(permissions["issues"].as_str(), Some("write"));
    for step in steps(job) {
        for (name, value) in step["env"].as_mapping().into_iter().flatten() {
            if name.as_str() == Some("GH_TOKEN") {
                assert_eq!(value.as_str(), Some("${{ github.token }}"));
            }
        }
    }
}

#[test]
fn the_report_job_runs_after_the_others_on_main_and_never_checks_out_a_branch_of_a_pull_request() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["history-report"];

    let needs: Vec<&str> = job["needs"]
        .as_sequence()
        .expect("the needs")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(needs, ["lint", "report", "agnix"]);
    let condition = job["if"].as_str().expect("a condition");
    assert!(condition.contains("!cancelled()"), "{condition}");
    assert!(
        condition.contains("github.ref == 'refs/heads/main'"),
        "{condition}"
    );
    let checkout = using(job, "actions/checkout@");
    assert_eq!(checkout["with"]["fetch-depth"].as_u64(), Some(0));
    assert!(checkout["with"]["ref"].is_null());
}

#[test]
fn the_report_reads_the_last_four_weeks_of_ci_and_of_this_workflow_with_a_cap_of_20() {
    let workflow = scheduled();

    assert_eq!(workflow["env"]["HISTORY_DAYS"].as_str(), Some("28"));
    assert_eq!(workflow["env"]["REPORT_MAX_FINDINGS"].as_str(), Some("20"));
    assert_eq!(
        workflow["env"]["REPORT_TITLE"].as_str(),
        Some("Weekly review report")
    );
    let job = &workflow["jobs"]["history-report"];
    let download = step_run(job, "Download the history of CI");
    assert!(download.contains("for workflow in ci.yml scheduled-review.yml"));
    assert!(download.contains("branch=$BRANCH"), "{download}");
    assert!(download.contains(r#"select(.head_repository.full_name == $repo)"#));
    assert!(download.contains(
        r#"select(.event == "push" or .event == "schedule" or .event == "workflow_dispatch")"#
    ));
    assert!(
        download.contains("git merge-base --is-ancestor"),
        "{download}"
    );
    assert!(download.contains("botica-history-"), "{download}");
    let read = step_run(job, "Read the history");
    assert!(read.contains(r#"--days "$HISTORY_DAYS""#), "{read}");
    assert!(read.contains("--format markdown"), "{read}");
    assert!(
        read.contains(r#"--from "$RUNNER_TEMP/ci-history""#),
        "{read}"
    );
}

#[test]
fn the_text_of_the_report_reaches_the_issue_through_a_file_and_never_through_the_shell() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["history-report"];

    let issue = step_run(job, "Open or update the issue");
    assert!(
        issue.contains(r#"--body-file "$RUNNER_TEMP/issue.md""#),
        "{issue}"
    );
    assert!(!issue.contains("--body "), "{issue}");
    assert!(issue.contains("gh issue edit"), "{issue}");
    assert!(issue.contains("gh issue create"), "{issue}");
    for step in steps(job) {
        let run = step["run"].as_str().unwrap_or_default();
        assert!(
            !run.contains("${{"),
            "an expression inside a script of the job: {run}"
        );
    }
}
