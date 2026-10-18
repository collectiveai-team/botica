//! `ci-automerge-public` and `ci-automerge-private`: the workflows that merge the Dependabot
//! pull requests that are patch updates or fix an open security alert.
//!
//! Sources: `collectiveai-team/coro` 58ae93f (`dependabot-automerge.yml`, a public repository
//! that has branch protection) and `collectiveai-team/collectiveai-web` 7a174d6
//! (`dependabot-automerge-label.yml` and `dependabot-automerge-run.yml`, a private repository on
//! the Free plan, where `gh pr merge --auto` merges at once). The minor-update lane of web is
//! not taken: `dependency-type` reports `direct:development` for a package that a dependency
//! group lists, even when it is also a production dependency.

mod common;

use botica_catalog::Recipes;
use common::workflows::{has_trigger, workflow};
use serde_yaml::Value;

const PUBLIC: &str = "files/github/workflows/dependabot-automerge.yml";
const LABEL: &str = "files/github/workflows/dependabot-automerge-label.yml";
const RUN: &str = "files/github/workflows/dependabot-automerge-run.yml";

fn recipes() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

fn paths(recipe: &str) -> Vec<String> {
    recipes()
        .recipe(recipe)
        .unwrap_or_else(|| panic!("no recipe {recipe}"))
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect()
}

fn commands(workflow: &Value) -> Vec<String> {
    workflow["jobs"]
        .as_mapping()
        .expect("the jobs")
        .values()
        .flat_map(|job| job["steps"].as_sequence().expect("the steps"))
        .filter_map(|step| step["run"].as_str().map(str::to_owned))
        .collect()
}

fn step_text(workflow: &Value) -> String {
    serde_yaml::to_string(&workflow["jobs"]).expect("YAML")
}

#[test]
fn the_public_recipe_adds_one_workflow_next_to_ci() {
    let recipes = recipes();

    assert_eq!(
        paths("ci-automerge-public"),
        [".github/workflows/dependabot-automerge.yml"]
    );
    assert_eq!(
        recipes.recipe("ci-automerge-public").expect("it").extends,
        ["ci"]
    );
}

#[test]
fn the_private_recipe_adds_the_workflow_that_labels_and_the_one_that_merges() {
    let recipes = recipes();

    assert_eq!(
        paths("ci-automerge-private"),
        [
            ".github/workflows/dependabot-automerge-label.yml",
            ".github/workflows/dependabot-automerge-run.yml"
        ]
    );
    assert_eq!(
        recipes.recipe("ci-automerge-private").expect("it").extends,
        ["ci"]
    );
}

#[test]
fn only_a_pull_request_of_dependabot_is_looked_at() {
    for (recipe, template) in [
        ("ci-automerge-public", PUBLIC),
        ("ci-automerge-private", LABEL),
    ] {
        let wf = workflow(recipe, template);

        assert!(has_trigger(&wf, "pull_request_target"), "{template}");
        for job in wf["jobs"].as_mapping().expect("the jobs").values() {
            assert_eq!(
                job["if"].as_str(),
                Some("github.event.pull_request.user.login == 'dependabot[bot]'"),
                "{template}"
            );
        }
    }
}

#[test]
fn no_automerge_workflow_checks_out_or_runs_the_code_of_the_pull_request() {
    for (recipe, template) in [
        ("ci-automerge-public", PUBLIC),
        ("ci-automerge-private", LABEL),
        ("ci-automerge-private", RUN),
    ] {
        let wf = workflow(recipe, template);

        assert!(
            !step_text(&wf).contains("actions/checkout"),
            "{template} checks out"
        );
    }
}

#[test]
fn only_a_patch_update_or_an_open_security_alert_is_easy() {
    for (recipe, template) in [
        ("ci-automerge-public", PUBLIC),
        ("ci-automerge-private", LABEL),
    ] {
        let script = commands(&workflow(recipe, template)).join("\n");

        assert!(script.contains("version-update:semver-patch"), "{template}");
        assert!(script.contains("\"$ALERT_STATE\" = \"OPEN\""), "{template}");
        assert!(!script.contains("semver-minor"), "{template}");
        assert!(!script.contains("semver-major"), "{template}");
    }
}

#[test]
fn every_gh_call_of_the_public_workflow_names_the_repository() {
    let script = commands(&workflow("ci-automerge-public", PUBLIC)).join("\n");

    let calls: Vec<&str> = script
        .lines()
        .filter(|line| line.trim_start().starts_with("gh "))
        .collect();

    assert!(calls.len() >= 3, "{calls:?}");
    for call in calls {
        assert!(call.contains("--repo \"$REPO\""), "{call}");
    }
}

#[test]
fn the_public_workflow_queues_the_native_auto_merge() {
    let script = commands(&workflow("ci-automerge-public", PUBLIC)).join("\n");

    assert!(
        script.contains("gh pr merge \"$PR_URL\" --repo \"$REPO\" --auto --squash --delete-branch"),
        "{script}"
    );
}

#[test]
fn the_workflow_that_labels_never_merges() {
    let label = workflow("ci-automerge-private", LABEL);

    for command in commands(&label) {
        assert!(!command.contains("gh pr merge"), "{command}");
    }
}

#[test]
fn the_workflow_that_merges_waits_for_the_workflows_of_ci_and_does_not_use_auto_merge() {
    let run = workflow("ci-automerge-private", RUN);

    assert!(has_trigger(&run, "workflow_run"));
    assert_eq!(
        run["on"]["workflow_run"]["types"][0].as_str(),
        Some("completed")
    );
    let job = &run["jobs"]["merge"];
    assert_eq!(
        job["if"].as_str(),
        Some("github.event.workflow_run.conclusion == 'success'")
    );
    let script = commands(&run).join("\n");
    assert!(script.contains("gh pr merge \"$NUMBER\" --repo \"$REPO\" --squash --delete-branch"));
    assert!(!script.contains("--auto"), "{script}");
    for call in script.lines().filter(|line| line.contains("gh pr ")) {
        assert!(call.contains("--repo \"$REPO\""), "{call}");
    }
}

fn name_of(recipe: &str, template: &str) -> String {
    workflow(recipe, template)["name"]
        .as_str()
        .expect("a name")
        .to_owned()
}

#[test]
fn the_workflows_that_it_waits_for_are_the_ones_that_ci_and_ci_python_add() {
    let run = workflow("ci-automerge-private", RUN);
    let tracked: Vec<String> = run["on"]["workflow_run"]["workflows"]
        .as_sequence()
        .expect("the workflows")
        .iter()
        .map(|name| name.as_str().expect("a name").to_owned())
        .collect();
    let script = commands(&run).join("\n");

    let shipped = [
        name_of("ci-python", "files/github/workflows/tests.yml"),
        name_of("ci-python", "files/github/workflows/osv-scanner.yml"),
        name_of("ci", "files/github/workflows/zizmor.yml"),
        name_of("ci", "files/github/workflows/conventional-commits.yml"),
        name_of("ci", "files/github/workflows/commit-policy.yml"),
    ];

    assert_eq!(tracked, shipped);
    for name in &tracked {
        assert!(
            script.contains(&format!("\"{name}\"")),
            "{name} is not checked"
        );
    }
}
