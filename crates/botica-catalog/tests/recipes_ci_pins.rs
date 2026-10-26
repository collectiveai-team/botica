//! The versions that the workflows of the built-in recipes of CI pin.
//!
//! The floors are the versions that the workflows of `collectiveai-team/collectiveai-web`
//! (`build.yml`, `zizmor.yml`) and `collectiveai-team/coro` (`docker.yml`) run.

mod common;

use common::workflows::{text, version};

fn workflow(recipe: &str, name: &str) -> String {
    text(recipe, &format!("files/github/workflows/{name}"))
}

fn pins(text: &str, action: &str) -> Vec<Vec<u32>> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("uses:"))
        .filter_map(|uses| uses.trim().strip_prefix(&format!("{action}@")))
        .map(version)
        .collect()
}

fn quoted_versions(text: &str) -> Vec<Vec<u32>> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("version: \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .map(version)
        .collect()
}

const UV_WORKFLOWS: [(&str, &str); 4] = [
    ("ci", "zizmor.yml"),
    ("ci-python", "tests.yml"),
    ("ci-publish", "release.yml"),
    ("ci-publish", "pypi.yml"),
];

#[test]
fn every_workflow_that_installs_uv_pins_setup_uv_at_or_above_10_2_0() {
    for (recipe, name) in UV_WORKFLOWS {
        let found = pins(&workflow(recipe, name), "astral-sh/setup-uv");

        assert_eq!(found.len(), 1, "{name} pins setup-uv once");
        assert!(found[0] >= vec![10, 2, 0], "{name} pins {:?}", found[0]);
    }
}

#[test]
fn every_workflow_that_installs_uv_pins_uv_at_or_above_0_12_3() {
    for (recipe, name) in UV_WORKFLOWS {
        let found = quoted_versions(&workflow(recipe, name));

        assert_eq!(found.len(), 1, "{name} pins uv once");
        assert!(found[0] >= vec![0, 12, 3], "{name} pins {:?}", found[0]);
    }
}

#[test]
fn the_docker_workflow_pins_the_major_versions_that_coro_runs() {
    let text = workflow("ci-docker", "docker.yml");

    for (action, major) in [
        ("docker/login-action", 4),
        ("docker/metadata-action", 6),
        ("docker/setup-buildx-action", 4),
        ("docker/build-push-action", 7),
    ] {
        let found = pins(&text, action);

        assert_eq!(found.len(), 1, "{action} is pinned once");
        assert!(found[0] >= vec![major], "{action} pins {:?}", found[0]);
    }
}

#[test]
fn the_docker_workflow_attests_the_pushed_digest_in_a_public_repository_only() {
    let text = workflow("ci-docker", "docker.yml");
    let step = text
        .split("- name: ")
        .find(|step| step.contains("uses: actions/attest@"))
        .expect("a step attests the image");

    assert_eq!(pins(&text, "actions/attest"), [vec![4]]);
    assert!(
        step.contains("if: ${{ !github.event.repository.private }}"),
        "{step}"
    );
    assert!(
        step.contains("subject-digest: ${{ steps.push.outputs.digest }}"),
        "{step}"
    );
    for permission in [
        "id-token: write",
        "attestations: write",
        "artifact-metadata: write",
    ] {
        assert!(text.contains(permission), "{permission}");
    }
}
