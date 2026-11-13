//! `.github/workflows/scheduled-review.yml` of this repository: the weekly checks of `main`.
//!
//! A `schedule` runs on the default branch only, so no pull request can run the workflow. These
//! tests read the file and hold the properties that a run would otherwise be the first to show.

mod common;

use std::fs;
use std::path::Path;

use common::workflows::{repository_workflow as read, steps};
use serde_yaml::Value;

fn scheduled() -> Value {
    read("scheduled-review.yml")
}

fn runs(job: &Value) -> Vec<&str> {
    steps(job)
        .iter()
        .filter_map(|s| s["run"].as_str())
        .collect()
}

fn using<'a>(job: &'a Value, prefix: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|s| s["uses"].as_str().is_some_and(|u| u.starts_with(prefix)))
        .unwrap_or_else(|| panic!("no step uses {prefix}"))
}

#[test]
fn it_runs_weekly_and_on_demand_and_never_on_a_pull_request() {
    let workflow = scheduled();
    let on = workflow["on"].as_mapping().expect("the triggers");

    assert!(on.contains_key("workflow_dispatch"));
    let cron = on["schedule"][0]["cron"].as_str().expect("a cron");
    let fields: Vec<&str> = cron.split(' ').collect();
    assert_eq!(fields.len(), 5, "{cron}");
    assert!(fields[2] == "*" && fields[3] == "*", "weekly: {cron}");
    assert!(
        fields[4].parse::<u8>().is_ok(),
        "one day of the week: {cron}"
    );
    assert_eq!(on.len(), 2, "no other trigger");
}

#[test]
fn it_only_reads_and_holds_no_secret() {
    let workflow = scheduled();
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/scheduled-review.yml"),
    )
    .expect("the workflow");

    let permissions = workflow["permissions"].as_mapping().expect("permissions");
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions["contents"].as_str(), Some("read"));
    for (name, job) in workflow["jobs"].as_mapping().expect("the jobs") {
        assert!(
            job["permissions"].is_null(),
            "{name:?} widens the permissions"
        );
    }
    assert!(!text.contains("secrets."));
    assert!(!text.contains("GITHUB_TOKEN"));
    assert!(!text.contains("gh issue"));
}

#[test]
fn every_job_stays_on_main_and_every_checkout_drops_its_credentials() {
    // `plan` only runs `jq`, so it has no checkout.
    let workflow = scheduled();

    let on_main = |job: &Value| {
        job["if"]
            .as_str()
            .is_some_and(|c| c.contains("github.ref == 'refs/heads/main'"))
    };
    for (name, job) in workflow["jobs"].as_mapping().expect("the jobs") {
        // A job whose `needs` is skipped is skipped too, so `mutants` follows `plan`.
        let needed = job["needs"]
            .as_str()
            .is_some_and(|needed| on_main(&workflow["jobs"][needed]));
        assert!(on_main(job) || needed, "{name:?} runs off main");
        for step in steps(job) {
            if step["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout@"))
            {
                assert_eq!(
                    step["with"]["persist-credentials"].as_bool(),
                    Some(false),
                    "{name:?}"
                );
            }
        }
    }
}

#[test]
fn the_lint_records_every_file_of_main_and_keeps_the_history() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["lint"];

    assert!(runs(job).contains(&"target/debug/botica lint --record"));
    assert!(runs(job).contains(&"cargo build --locked -p botica-cli"));
    let upload = using(job, "actions/upload-artifact@");
    assert_eq!(upload["if"].as_str(), Some("${{ !cancelled() }}"));
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("botica-history-scheduled-lint")
    );
    assert_eq!(upload["with"]["include-hidden-files"].as_bool(), Some(true));
    let paths = upload["with"]["path"].as_str().expect("the paths");
    assert!(paths.lines().any(|l| l == ".botica/history/runs.jsonl"));
    assert!(paths.lines().any(|l| l == ".botica/history/findings.jsonl"));
}

#[test]
fn the_history_artifacts_follow_the_name_that_history_fetch_reads() {
    let workflow = scheduled();

    for (name, job) in workflow["jobs"].as_mapping().expect("the jobs") {
        for step in steps(job) {
            let uploaded = step["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/upload-artifact@"));
            let held = step["with"]["path"]
                .as_str()
                .is_some_and(|p| p.contains(".botica/history"));
            if uploaded && held {
                let artifact = step["with"]["name"].as_str().expect("a name");
                assert!(
                    artifact.starts_with("botica-history-"),
                    "{name:?}: {artifact}"
                );
            }
        }
    }
}

#[test]
fn the_shards_cover_the_run_and_agree_on_the_denominator() {
    let workflow = scheduled();
    let shards: u64 = workflow["env"]["MUTANT_SHARDS"]
        .as_str()
        .expect("the count")
        .parse()
        .expect("a number");
    let mutants = &workflow["jobs"]["mutants"];

    let matrix: Vec<u64> = mutants["strategy"]["matrix"]["shard"]
        .as_sequence()
        .expect("the shards")
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    assert_eq!(matrix, (0..shards).collect::<Vec<_>>());
    assert_eq!(mutants["needs"].as_str(), Some("plan"));
    assert_eq!(mutants["strategy"]["fail-fast"].as_bool(), Some(false));
    let cargo = runs(mutants)
        .into_iter()
        .find(|r| r.contains("cargo mutants"))
        .expect("the step of cargo-mutants");
    assert!(
        cargo.contains(r#"--shard "$SHARD/$MUTANT_SHARDS""#),
        "{cargo}"
    );
    assert!(cargo.contains("--test-workspace=true"), "{cargo}");
    assert!(cargo.contains("0|2|3) ;;"), "{cargo}");
    assert!(
        cargo.contains(r#"--in-diff "$RUNNER_TEMP/week/week.diff""#),
        "{cargo}"
    );
}

#[test]
fn the_mutants_are_those_of_the_rust_lines_that_main_changed_in_the_last_week() {
    let workflow = scheduled();
    let plan = &workflow["jobs"]["plan"];
    let mutants = &workflow["jobs"]["mutants"];

    let script = runs(plan).join("\n");
    assert!(script.contains(r#"--before="7 days ago""#), "{script}");
    assert!(
        script.contains(r#"git diff "$base" HEAD -- '*.rs' > week.diff"#),
        "{script}"
    );
    assert!(script.contains("rust=false"), "{script}");
    assert_eq!(
        using(plan, "actions/checkout@")["with"]["fetch-depth"].as_u64(),
        Some(0),
        "the commit of 7 days ago needs the history"
    );
    assert_eq!(
        mutants["if"].as_str(),
        Some("needs.plan.outputs.rust == 'true'")
    );
    assert_eq!(
        using(mutants, "actions/download-artifact@")["with"]["name"].as_str(),
        using(plan, "actions/upload-artifact@")["with"]["name"].as_str()
    );
    assert!(workflow["jobs"]["report"]["if"]
        .as_str()
        .is_some_and(|c| c.contains("needs.plan.outputs.rust == 'true'")));
}

#[test]
fn a_shard_does_not_stop_the_others_and_has_a_cap() {
    let workflow = scheduled();
    let mutants = &workflow["jobs"]["mutants"];

    assert_eq!(mutants["continue-on-error"].as_bool(), Some(true));
    assert!(mutants["timeout-minutes"]
        .as_u64()
        .is_some_and(|m| m <= 360));
    let upload = using(mutants, "actions/upload-artifact@");
    assert_eq!(upload["if"].as_str(), Some("${{ always() }}"));
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("mutants-shard-${{ matrix.shard }}")
    );
}

#[test]
fn it_installs_the_cargo_mutants_that_the_pull_request_job_installs() {
    let install = |file: &str, job: &str| {
        let workflow = read(file);
        runs(&workflow["jobs"][job])
            .into_iter()
            .find(|r| r.starts_with("cargo install cargo-mutants"))
            .map(str::to_owned)
            .unwrap_or_else(|| panic!("{file} installs no cargo-mutants"))
    };

    assert_eq!(
        install("scheduled-review.yml", "mutants"),
        install("ci.yml", "mutants")
    );
}

#[test]
fn the_report_joins_every_shard_and_fails_when_one_left_nothing() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["report"];

    assert_eq!(
        job["needs"].as_sequence().map(Vec::len),
        Some(2),
        "plan and mutants"
    );
    assert!(job["if"]
        .as_str()
        .is_some_and(|c| c.starts_with("${{ !cancelled()")));
    let download = using(job, "actions/download-artifact@");
    assert_eq!(
        download["with"]["pattern"].as_str(),
        Some("mutants-shard-*")
    );
    let runs = runs(job);
    assert!(runs
        .iter()
        .any(|r| r.contains("jq -s '{outcomes: (map(.outcomes) | add)}'")));
    let lint = runs
        .iter()
        .find(|r| r.contains("botica lint --mutants mutants-all.json"))
        .expect("the step of botica");
    assert!(
        lint.contains("--preview --select mutant-survived"),
        "{lint}"
    );
    assert!(lint.contains("--record"), "{lint}");
    assert!(runs
        .iter()
        .any(|r| r.contains(r#"[ "$FOUND" -ne "$MUTANT_SHARDS" ]"#)));
}

#[test]
fn the_report_keeps_its_history_and_the_last_check_runs_after_a_failure() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["report"];
    let upload = using(job, "actions/upload-artifact@");

    assert_eq!(upload["if"].as_str(), Some("${{ !cancelled() }}"));
    assert_eq!(
        upload["with"]["name"].as_str(),
        Some("botica-history-scheduled-mutants")
    );
    assert_eq!(upload["with"]["include-hidden-files"].as_bool(), Some(true));
    let paths = upload["with"]["path"].as_str().expect("the paths");
    for path in [
        "mutants-all.json",
        ".botica/history/runs.jsonl",
        ".botica/history/findings.jsonl",
    ] {
        assert!(paths.lines().any(|l| l == path), "{path}: {paths}");
    }
    let last = steps(job).last().expect("a last step");
    assert_eq!(last["if"].as_str(), Some("${{ !cancelled() }}"));
}

#[test]
fn only_the_shards_continue_on_error_and_they_leave_runners_for_pull_requests() {
    let workflow = scheduled();

    for (name, job) in workflow["jobs"].as_mapping().expect("the jobs") {
        let tolerant = job["continue-on-error"].as_bool() == Some(true);
        assert_eq!(tolerant, name.as_str() == Some("mutants"), "{name:?}");
    }
    let parallel = workflow["jobs"]["mutants"]["strategy"]["max-parallel"]
        .as_u64()
        .expect("max-parallel");
    assert!((1..=4).contains(&parallel), "{parallel}");
}

#[test]
fn ci_runs_the_tests_of_this_workflow_on_a_pull_request_that_edits_it() {
    let ci = read("ci.yml");
    let paths: Vec<&str> = ci["on"]["pull_request"]["paths"]
        .as_sequence()
        .expect("the paths")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    for path in [".github/workflows/scheduled-review.yml", "prek.toml"] {
        assert!(paths.contains(&path), "{path}");
    }
}

#[test]
fn agnix_runs_through_the_hook_of_prek_toml_over_every_file() {
    let workflow = scheduled();
    let job = &workflow["jobs"]["agnix"];

    assert_eq!(runs(job), ["uvx prek run agnix --all-files --verbose"]);
    let hooks = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prek.toml"))
        .expect("prek.toml");
    assert!(hooks.contains(r#"id = "agnix""#));
}

#[test]
fn every_action_is_pinned_to_a_tag_like_the_rest_of_the_repository() {
    let workflow = scheduled();

    for (name, job) in workflow["jobs"].as_mapping().expect("the jobs") {
        for step in steps(job) {
            if let Some(uses) = step["uses"].as_str() {
                let tag = uses.rsplit('@').next().expect("a reference");
                let numbers = tag.strip_prefix('v').unwrap_or_default();
                assert!(
                    !numbers.is_empty() && numbers.split('.').all(|n| n.parse::<u32>().is_ok()),
                    "{name:?}: {uses}"
                );
            }
        }
    }
}
