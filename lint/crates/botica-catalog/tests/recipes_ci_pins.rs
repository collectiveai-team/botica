//! The versions that the workflows of the built-in recipes of CI pin.
//!
//! The floors are the versions that the workflows of `collectiveai-team/collectiveai-web`
//! (`build.yml`, `zizmor.yml`) and `collectiveai-team/coro` (`docker.yml`) run.

use botica_catalog::Recipes;

fn workflow(recipe: &str, name: &str) -> String {
    Recipes::embedded()
        .expect("the built-in recipes load")
        .template(recipe, &format!("files/github/workflows/{name}"))
        .unwrap_or_else(|| panic!("{recipe} has no {name}"))
        .to_owned()
}

fn version(text: &str) -> Vec<u32> {
    text.trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().expect("a number"))
        .collect()
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
