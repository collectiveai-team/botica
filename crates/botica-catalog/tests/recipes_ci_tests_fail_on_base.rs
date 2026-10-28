//! The `tests-fail-on-base-python.yml` workflow of `ci-tests-fail-on-base-python` (CES-124).

mod common;

use common::workflows::workflow;
use serde_yaml::Value;

const RECIPE: &str = "ci-tests-fail-on-base-python";
const TEMPLATE: &str = "files/github/workflows/tests-fail-on-base-python.yml";

fn steps(workflow: &Value) -> &Vec<Value> {
    workflow["jobs"]["tests"]["steps"]
        .as_sequence()
        .expect("the steps")
}

#[test]
fn adding_or_removing_the_label_runs_the_check_again() {
    let workflow = workflow(RECIPE, TEMPLATE);

    let types: Vec<&str> = workflow["on"]["pull_request"]["types"]
        .as_sequence()
        .expect("the types")
        .iter()
        .filter_map(Value::as_str)
        .collect();

    assert_eq!(
        types,
        ["opened", "synchronize", "reopened", "labeled", "unlabeled"]
    );
    assert_eq!(
        workflow["env"]["LABEL"].as_str(),
        Some("tests-pass-on-base")
    );
}

#[test]
fn the_job_reads_the_whole_history_and_keeps_no_credentials() {
    let workflow = workflow(RECIPE, TEMPLATE);

    let checkout = &steps(&workflow)[0];

    assert_eq!(workflow["permissions"]["contents"].as_str(), Some("read"));
    assert_eq!(
        workflow["permissions"].as_mapping().map(|p| p.len()),
        Some(1)
    );
    assert_eq!(checkout["with"]["fetch-depth"].as_u64(), Some(0));
    assert_eq!(
        checkout["with"]["persist-credentials"].as_bool(),
        Some(false)
    );
}

#[test]
fn the_title_of_the_pull_request_reaches_the_script_through_the_environment() {
    let workflow = workflow(RECIPE, TEMPLATE);

    let run = steps(&workflow).last().expect("the step of the script");

    assert_eq!(
        run["env"]["PR_TITLE"].as_str(),
        Some("${{ github.event.pull_request.title }}")
    );
    for step in steps(&workflow) {
        let script = step["run"].as_str().unwrap_or_default();
        assert!(
            !script.contains("${{"),
            "a `run:` interpolates an expression: {script}"
        );
    }
}

#[test]
fn the_workflow_runs_the_script_that_the_recipe_installs() {
    let workflow = workflow(RECIPE, TEMPLATE);
    let recipes = botica_catalog::Recipes::embedded().expect("the built-in recipes load");
    let recipe = recipes.recipe(RECIPE).expect("the recipe");

    let run = steps(&workflow).last().expect("the step of the script")["run"]
        .as_str()
        .expect("run");

    let installed: Vec<&str> = recipe.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        run,
        "uv run --no-project .github/scripts/tests_fail_on_base.py"
    );
    assert!(installed.contains(&".github/scripts/tests_fail_on_base.py"));
    assert!(installed.contains(&".github/scripts/tests_fail_on_base_core.py"));
    assert!(installed.contains(&".github/scripts/tests_fail_on_base_pytest.py"));
}
