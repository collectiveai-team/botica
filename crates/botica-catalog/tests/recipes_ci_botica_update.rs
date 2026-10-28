//! `ci-botica-update`: the weekly workflow that moves a project to the newest release of botica
//! and opens a pull request (discussion #261, second step).

mod common;

use botica_catalog::Recipes;
use common::workflows::{has_trigger, workflow};
use serde_yaml::Value;

const WORKFLOW: &str = "files/github/workflows/botica-update.yml";

fn steps(wf: &Value) -> Vec<Value> {
    wf["jobs"]["update"]["steps"]
        .as_sequence()
        .expect("the steps")
        .clone()
}

fn run_of(wf: &Value, name: &str) -> String {
    steps(wf)
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .and_then(|step| step["run"].as_str())
        .unwrap_or_else(|| panic!("no step {name}"))
        .to_owned()
}

#[test]
fn the_recipe_adds_the_workflow_and_the_script_and_extends_nothing() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let recipe = recipes.recipe("ci-botica-update").expect("the recipe");

    let paths: Vec<&str> = recipe.files.iter().map(|file| file.path.as_str()).collect();

    assert_eq!(
        paths,
        [
            ".github/workflows/botica-update.yml",
            ".github/scripts/botica_update.py"
        ]
    );
    assert!(recipe.extends.is_empty());
}

#[test]
fn the_workflow_runs_weekly_and_by_hand_with_read_access_and_no_stored_credentials() {
    let wf = workflow("ci-botica-update", WORKFLOW);

    assert!(has_trigger(&wf, "schedule"));
    assert!(has_trigger(&wf, "workflow_dispatch"));
    assert_eq!(wf["on"].as_mapping().expect("the triggers").len(), 2);
    assert_eq!(wf["permissions"]["contents"].as_str(), Some("read"));
    assert_eq!(
        wf["jobs"]["update"]["permissions"]["contents"].as_str(),
        Some("read")
    );
    let checkout = &steps(&wf)[0];
    assert!(checkout["uses"]
        .as_str()
        .is_some_and(|uses| uses.starts_with("actions/checkout@")));
    assert_eq!(
        checkout["with"]["persist-credentials"].as_bool(),
        Some(false)
    );
}

#[test]
fn prek_moves_only_the_mirror_with_the_cooldown_of_the_project() {
    let wf = workflow("ci-botica-update", WORKFLOW);

    let run = run_of(&wf, "Move the rev of the mirror of botica");

    assert!(run.starts_with("uvx prek@"), "{run}");
    assert!(run.contains(" update"), "{run}");
    assert!(
        run.contains("--repo https://github.com/collectiveai-team/botica-pre-commit"),
        "{run}"
    );
    assert!(run.contains("--cooldown-days 7"), "{run}");
    assert!(run.contains("--exclude-tag '*-rc*'"), "{run}");
}

#[test]
fn the_installed_script_writes_the_version_and_the_secret_opens_the_pull_request() {
    let wf = workflow("ci-botica-update", WORKFLOW);

    let align = run_of(
        &wf,
        "Write the version in the hooks, the workflows and the configuration",
    );
    let open = steps(&wf)
        .into_iter()
        .find(|step| step["name"].as_str() == Some("Open or update the pull request"))
        .expect("the step that opens the pull request");

    assert!(align.contains("uv run --no-project .github/scripts/botica_update.py"));
    assert_eq!(
        open["env"]["GH_TOKEN"].as_str(),
        Some("${{ secrets.BOTICA_UPDATE_TOKEN }}")
    );
    let script = open["run"].as_str().expect("the script");
    assert!(script.contains("the secret BOTICA_UPDATE_TOKEN is not set"));
    assert!(script.contains("git push --force"));
    assert!(script.contains(" chore/botica-update\n"));
}
