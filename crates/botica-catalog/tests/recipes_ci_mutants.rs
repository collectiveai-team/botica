//! `ci-mutants-rust`, `ci-mutants-go` and `ci-mutants-node`: a workflow per language that tests the
//! mutants of the code a pull request changes and never blocks it.
//!
//! The job of `ci-mutants-rust` is the job `mutants` of the `ci.yml` of this repository, with the
//! botica of the release instead of a build of the workspace.

mod common;

use botica_catalog::Recipes;
use common::workflows::{text, workflow};
use serde_yaml::Value;

const RUST: (&str, &str) = (
    "ci-mutants-rust",
    "files/github-rust/workflows/mutants-rust.yml",
);
const GO: (&str, &str) = ("ci-mutants-go", "files/github-go/workflows/mutants-go.yml");
const NODE: (&str, &str) = (
    "ci-mutants-node",
    "files/github-node/workflows/mutants-node.yml",
);

fn job((recipe, template): (&str, &str)) -> Value {
    workflow(recipe, template)["jobs"]["mutants"].clone()
}

fn runs(job: &Value) -> Vec<String> {
    job["steps"]
        .as_sequence()
        .expect("the steps")
        .iter()
        .filter_map(|step| step["run"].as_str())
        .map(str::to_owned)
        .collect()
}

#[test]
fn each_recipe_writes_one_workflow_and_extends_nothing() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    for ((recipe, template), path) in [
        (RUST, ".github/workflows/mutants-rust.yml"),
        (GO, ".github/workflows/mutants-go.yml"),
        (NODE, ".github/workflows/mutants-node.yml"),
    ] {
        let found = recipes.recipe(recipe).expect("the recipe");
        let files: Vec<(&str, &str)> = found
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.template.as_str()))
            .collect();
        assert_eq!(files, [(path, template)], "{recipe}");
        assert!(found.extends.is_empty(), "{recipe}");
        assert!(found.tools.is_empty(), "{recipe}");
    }
}

#[test]
fn every_job_runs_on_a_pull_request_without_blocking_it_and_within_a_cap() {
    for pair in [RUST, GO, NODE] {
        let written = workflow(pair.0, pair.1);
        let job = &written["jobs"]["mutants"];

        let events = written["on"]["pull_request"]["types"]
            .as_sequence()
            .expect("PR events");
        assert!(
            events
                .iter()
                .any(|event| event.as_str() == Some("ready_for_review")),
            "{}",
            pair.0
        );
        assert_eq!(job["needs"].as_str(), Some("review-policy"));
        assert!(written["jobs"]["review-policy"]["if"]
            .as_str()
            .unwrap()
            .contains("draft == false"));
        assert_eq!(written["permissions"]["contents"].as_str(), Some("read"));
        assert_eq!(job["continue-on-error"].as_bool(), Some(true), "{}", pair.0);
        assert!(job["timeout-minutes"].as_u64().is_some(), "{}", pair.0);
        let checkout = &job["steps"][0];
        assert!(checkout["uses"]
            .as_str()
            .is_some_and(|u| u.starts_with("actions/checkout@")));
        assert_eq!(
            checkout["with"]["persist-credentials"].as_bool(),
            Some(false)
        );
    }
}

#[test]
fn no_workflow_reads_a_secret() {
    for (recipe, template) in [RUST, GO, NODE] {
        assert!(!text(recipe, template).contains("secrets."), "{recipe}");
    }
}

#[test]
fn rust_tests_the_mutants_of_the_diff_and_reports_the_survivors_with_botica() {
    let runs = runs(&job(RUST));

    assert!(runs.contains(&"cargo install cargo-mutants --version 27.1.0 --locked".to_owned()));
    let mutants = runs
        .iter()
        .find(|run| run.contains("cargo mutants"))
        .expect("the step of cargo-mutants");
    assert!(
        mutants.contains(r#"git diff "$BASE_SHA...HEAD" > "$RUNNER_TEMP/pr.diff""#),
        "{mutants}"
    );
    assert!(
        mutants.contains(r#"--in-diff "$RUNNER_TEMP/pr.diff""#),
        "{mutants}"
    );
    assert!(mutants.contains("0|2|3) ;;"), "{mutants}");
    let lint = runs
        .iter()
        .find(|run| run.contains(" lint --mutants "))
        .expect("the step of botica");
    let release = format!("uvx botica@{} ", env!("CARGO_PKG_VERSION"));
    assert!(lint.starts_with(&release), "{lint}");
    for flag in [
        "--mutants mutants.out/outcomes.json",
        r#"--diff-file "$RUNNER_TEMP/pr.diff""#,
        "--preview --select mutant-survived",
        "--record",
    ] {
        assert!(lint.contains(flag), "{flag}: {lint}");
    }
}

#[test]
fn go_tests_the_mutants_of_the_diff_with_gremlins_and_uploads_its_report() {
    let job = job(GO);
    let runs = runs(&job);

    assert_eq!(
        runs,
        [
            "go install github.com/go-gremlins/gremlins/cmd/gremlins@v0.6.0",
            r#"gremlins unleash --diff "origin/$BASE_REF" --output gremlins.json --output-statuses lct"#,
        ]
    );
    assert_eq!(job["steps"][0]["with"]["fetch-depth"].as_u64(), Some(0));
    assert!(!runs.iter().any(|run| run.contains("botica@")));
    let upload = job["steps"]
        .as_sequence()
        .expect("the steps")
        .last()
        .cloned();
    let upload = upload.expect("the step of the artifact");
    assert_eq!(upload["with"]["path"].as_str(), Some("gremlins.json"));
}

#[test]
fn node_runs_the_stryker_of_the_project_incrementally_and_keeps_its_file_in_the_cache() {
    let written = workflow(NODE.0, NODE.1);
    let job = &written["jobs"]["mutants"];
    let steps = job["steps"].as_sequence().expect("the steps");

    assert_eq!(written["on"]["push"]["branches"][0].as_str(), Some("main"));
    let stryker = steps
        .iter()
        .find(|step| step["run"].as_str().is_some_and(|r| r.contains("stryker")))
        .expect("the step of Stryker");
    assert!(stryker["run"]
        .as_str()
        .is_some_and(|r| r.starts_with("pnpm exec stryker run --incremental ")));
    let inner = stryker["timeout-minutes"].as_u64().expect("a cap");
    assert!(inner < job["timeout-minutes"].as_u64().expect("a cap"));
    let restore = uses(steps, "actions/cache/restore@");
    let save = uses(steps, "actions/cache/save@");
    for step in [restore, save] {
        assert_eq!(
            step["with"]["path"].as_str(),
            Some("reports/stryker-incremental.json")
        );
    }
    assert_eq!(restore["with"]["key"], save["with"]["key"]);
    let fallbacks = restore["with"]["restore-keys"].as_str().expect("the keys");
    assert!(
        fallbacks.contains("stryker-incremental-${{ github.base_ref || github.ref_name }}-"),
        "{fallbacks}"
    );
    assert!(save["if"].as_str().is_some_and(|c| c.contains("always()")));
    assert!(!runs(job).iter().any(|run| run.contains("botica@")));
}

#[cfg(unix)]
#[test]
fn trusted_recipe_policy_runs_mutation_conservatively_or_uses_a_complete_plan() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("recipe-mutation-policy");
    fs::create_dir_all(root.join("bin")).unwrap();
    for (name, script) in [
        ("git", "#!/bin/sh\nexit 0\n"),
        (
            "uvx",
            r#"#!/bin/sh
case "$*" in
  *--help*) test "$FAKE_PLAN_MODE" != legacy; exit $? ;;
esac
case "$FAKE_PLAN_MODE" in
  failure) exit 2 ;;
  low) echo '{"status":"complete","mutation_required":false}' ;;
  *) echo '{"status":"complete","mutation_required":true}' ;;
esac
"#,
        ),
    ] {
        let file = root.join("bin").join(name);
        fs::write(&file, script).unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    for pair in [RUST, GO, NODE] {
        let written = workflow(pair.0, pair.1);
        let script = runs(&written["jobs"]["review-policy"])
            .into_iter()
            .find(|run| run.contains("review plan"))
            .unwrap();
        for (mode, expected) in [
            ("legacy", "run=true\n"),
            ("low", "run=false\n"),
            ("medium", "run=true\n"),
            ("failure", ""),
        ] {
            let output_path = root.join(format!("{}-{mode}.output", pair.0));
            fs::write(&output_path, "").unwrap();
            let output = Command::new("bash")
                .args(["-eo", "pipefail", "-c", &script])
                .env(
                    "PATH",
                    format!(
                        "{}:{}",
                        root.join("bin").display(),
                        std::env::var("PATH").unwrap_or_default()
                    ),
                )
                .env("RUNNER_TEMP", &root)
                .env("GITHUB_OUTPUT", &output_path)
                .env("BASE_SHA", "a".repeat(40))
                .env("HEAD_SHA", "b".repeat(40))
                .env("PR_NUMBER", "42")
                .env("FAKE_PLAN_MODE", mode)
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                mode != "failure",
                "{} {mode}",
                pair.0
            );
            assert_eq!(
                fs::read_to_string(output_path).unwrap(),
                expected,
                "{} {mode}",
                pair.0
            );
        }
    }
}

fn uses<'a>(steps: &'a [Value], prefix: &str) -> &'a Value {
    steps
        .iter()
        .find(|step| step["uses"].as_str().is_some_and(|u| u.starts_with(prefix)))
        .unwrap_or_else(|| panic!("no step uses {prefix}"))
}

#[test]
fn every_job_uploads_its_report_also_when_a_step_before_failed() {
    for (pair, artifact) in [
        (RUST, "botica-mutants"),
        (GO, "gremlins-mutants"),
        (NODE, "stryker-mutants"),
    ] {
        let job = job(pair);
        let steps = job["steps"].as_sequence().expect("the steps");
        let upload = uses(steps, "actions/upload-artifact@");

        assert_eq!(upload["if"].as_str(), Some("${{ always() }}"), "{}", pair.0);
        assert_eq!(upload["with"]["name"].as_str(), Some(artifact));
    }
}

#[test]
fn rust_diffs_against_the_base_and_keeps_the_history_in_its_artifact() {
    let job = job(RUST);
    let steps = job["steps"].as_sequence().expect("the steps");
    let upload = uses(steps, "actions/upload-artifact@");

    assert_eq!(steps[0]["with"]["fetch-depth"].as_u64(), Some(0));
    let paths = upload["with"]["path"].as_str().expect("the paths");
    for path in [
        "mutants.out/",
        ".botica/history/runs.jsonl",
        ".botica/history/findings.jsonl",
    ] {
        assert!(paths.lines().any(|line| line == path), "{path}: {paths}");
    }
    assert_eq!(upload["with"]["include-hidden-files"].as_bool(), Some(true));
    let report = steps
        .iter()
        .find(|step| {
            step["run"]
                .as_str()
                .is_some_and(|r| r.contains(" lint --mutants "))
        })
        .expect("the step of botica");
    assert_eq!(
        report["if"].as_str(),
        Some("${{ !cancelled() && hashFiles('mutants.out/outcomes.json') != '' }}")
    );
}
