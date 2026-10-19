//! `release.yml` and `pypi.yml` of `ci-publish`.
//!
//! `release.yml` starts `pypi.yml` with `workflow_dispatch`: the Release is created with
//! `GITHUB_TOKEN`, and GitHub does not start a workflow from an event that `GITHUB_TOKEN`
//! emits, so `release: published` never reaches `pypi.yml`. Source:
//! `collectiveai-team/coro` 58ae93f, `.github/workflows/release.yml`.

mod common;

use common::workflows::workflow;
use serde_yaml::Value;

fn release() -> Value {
    workflow("ci-publish", "files/github/workflows/release.yml")
}

fn steps(release: &Value) -> &Vec<Value> {
    release["jobs"]["release"]["steps"]
        .as_sequence()
        .expect("the steps of the release job")
}

fn position(steps: &[Value], name: &str) -> usize {
    steps
        .iter()
        .position(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no step named {name}"))
}

#[test]
fn the_release_job_may_start_a_workflow() {
    let release = release();

    let permissions = &release["jobs"]["release"]["permissions"];

    assert_eq!(permissions["contents"].as_str(), Some("write"));
    assert_eq!(permissions["actions"].as_str(), Some("write"));
}

#[test]
fn the_release_starts_pypi_after_it_creates_the_github_release() {
    let release = release();
    let steps = steps(&release);

    let created = position(steps, "Create GitHub Release");
    let dispatched = position(steps, "Dispatch PyPI publish");

    assert!(dispatched > created);
    assert_eq!(dispatched, steps.len() - 1);
}

#[test]
fn the_dispatch_passes_the_tag_as_the_ref_that_pypi_builds() {
    let release = release();
    let steps = steps(&release);

    let step = &steps[position(steps, "Dispatch PyPI publish")];

    let command = step["run"].as_str().expect("run");
    assert!(command.contains("gh workflow run pypi.yml"), "{command}");
    assert!(command.contains("-f ref=\"$REF_NAME\""), "{command}");
    assert_eq!(
        step["env"]["REF_NAME"].as_str(),
        Some("${{ github.ref_name }}")
    );
    assert_eq!(
        step["env"]["GH_TOKEN"].as_str(),
        Some("${{ secrets.GITHUB_TOKEN }}")
    );
}

#[test]
fn no_step_of_the_release_interpolates_an_expression_in_its_command() {
    let release = release();

    for step in steps(&release) {
        if let Some(command) = step["run"].as_str() {
            assert!(!command.contains("${{"), "{command}");
        }
    }
}

#[test]
fn pypi_can_be_started_with_the_ref_to_publish() {
    let pypi = workflow("ci-publish", "files/github/workflows/pypi.yml");

    let input = &pypi["on"]["workflow_dispatch"]["inputs"]["ref"];

    assert_eq!(input["required"].as_bool(), Some(true));
}
