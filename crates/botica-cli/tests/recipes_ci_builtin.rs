//! `botica recipe install` with the recipes of CI that are built into the binary.

mod common;

use std::fs;

use common::recipes::{project, recipe};
use common::refs::ok;

fn workflows(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.join(".github/workflows"))
        .expect("the folder")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn a_private_repository_gets_the_common_workflows_and_the_zizmor_config() {
    let dir = project("ci-private", None);

    let text = ok(&recipe(&dir, &["install", "repo-private"]));

    assert!(
        text.ends_with("repo-private: 4 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert_eq!(
        workflows(&dir),
        [
            "commit-policy.yml",
            "conventional-commits.yml",
            "zizmor.yml"
        ]
    );
    assert!(dir.join(".github/zizmor.yml").is_file());
}

#[test]
fn a_public_repository_gets_the_dependency_review_and_the_docker_workflows_too() {
    let dir = project("ci-public", None);

    let text = ok(&recipe(&dir, &["install", "repo-public"]));

    assert!(
        text.ends_with("repo-public: 6 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert_eq!(
        workflows(&dir),
        [
            "commit-policy.yml",
            "conventional-commits.yml",
            "dependency-review.yml",
            "docker.yml",
            "zizmor.yml"
        ]
    );
    let checked = ok(&recipe(&dir, &["check", "repo-public"]));
    assert!(
        checked.ends_with("repo-public: 6 checked, 0 differ\n"),
        "{checked}"
    );
}

#[test]
fn the_python_pieces_come_apart_from_the_common_ones() {
    let dir = project("ci-python", None);

    let text = ok(&recipe(&dir, &["install", "ci-python", "ci-publish"]));

    assert!(
        text.ends_with("ci-python, ci-publish: 10 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert!(dir.join(".github/dependabot.yml").is_file());
    assert!(dir.join(".github/workflows/tests.yml").is_file());
    assert!(dir.join(".github/workflows/pypi.yml").is_file());
}

#[test]
fn the_public_automerge_adds_one_workflow_to_the_common_ones() {
    let dir = project("ci-automerge-public", None);

    let text = ok(&recipe(&dir, &["install", "ci-automerge-public"]));

    assert!(
        text.ends_with("ci-automerge-public: 5 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert!(dir
        .join(".github/workflows/dependabot-automerge.yml")
        .is_file());
    let checked = ok(&recipe(&dir, &["check", "ci-automerge-public"]));
    assert!(
        checked.ends_with("ci-automerge-public: 5 checked, 0 differ\n"),
        "{checked}"
    );
}

#[test]
fn the_private_automerge_adds_the_workflow_that_labels_and_the_one_that_merges() {
    let dir = project("ci-automerge-private", None);

    let text = ok(&recipe(&dir, &["install", "ci-automerge-private"]));

    assert!(
        text.ends_with("ci-automerge-private: 6 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert!(dir
        .join(".github/workflows/dependabot-automerge-label.yml")
        .is_file());
    assert!(dir
        .join(".github/workflows/dependabot-automerge-run.yml")
        .is_file());
}

#[test]
fn the_botica_update_adds_its_workflow_and_the_script_that_it_runs() {
    let dir = project("ci-botica-update", None);

    let text = ok(&recipe(&dir, &["install", "ci-botica-update"]));

    assert!(
        text.ends_with("ci-botica-update: 2 added, 0 ok, 0 pending\n"),
        "{text}"
    );
    assert_eq!(workflows(&dir), ["botica-update.yml"]);
    assert!(dir.join(".github/scripts/botica_update.py").is_file());
    let checked = ok(&recipe(&dir, &["check", "ci-botica-update"]));
    assert!(
        checked.ends_with("ci-botica-update: 2 checked, 0 differ\n"),
        "{checked}"
    );
}
