//! `ci-node`: dependabot and osv-scanner for a Node project that uses pnpm.
//!
//! Source: `collectiveai-team/collectiveai-web` 7a174d6, `.github/dependabot.yml` and
//! `.github/workflows/osv-scanner.yml`. The `build.yml` of that repository is not a template: it
//! is specific to the project.

mod common;

use botica_catalog::Recipes;
use common::workflows::{osv_scanner_for, workflow};
use serde_yaml::Value;

const DEPENDABOT: &str = "files/github-node/dependabot.yml";
const OSV: &str = "files/github-node/workflows/osv-scanner.yml";

fn updates() -> Vec<Value> {
    workflow("ci-node", DEPENDABOT)["updates"]
        .as_sequence()
        .expect("the updates")
        .clone()
}

fn update<'a>(updates: &'a [Value], ecosystem: &str) -> &'a Value {
    updates
        .iter()
        .find(|u| u["package-ecosystem"].as_str() == Some(ecosystem))
        .unwrap_or_else(|| panic!("no update for {ecosystem}"))
}

#[test]
fn the_recipe_adds_dependabot_and_osv_scanner_next_to_ci() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    let node = recipes.recipe("ci-node").expect("ci-node");

    let paths: Vec<&str> = node.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            ".github/dependabot.yml",
            ".github/workflows/osv-scanner.yml"
        ]
    );
    assert_eq!(node.extends, ["ci"]);
}

#[test]
fn dependabot_keeps_the_actions_and_the_npm_packages_up_to_date_weekly_after_a_cooldown() {
    let updates = updates();

    let ecosystems: Vec<&str> = updates
        .iter()
        .map(|u| u["package-ecosystem"].as_str().expect("an ecosystem"))
        .collect();
    assert_eq!(ecosystems, ["github-actions", "npm"]);
    for update in &updates {
        assert_eq!(update["directory"].as_str(), Some("/"));
        assert_eq!(update["schedule"]["interval"].as_str(), Some("weekly"));
        assert_eq!(update["cooldown"]["default-days"].as_u64(), Some(7));
    }
}

#[test]
fn the_actions_come_in_one_pull_request() {
    let updates = updates();

    let patterns = &update(&updates, "github-actions")["groups"]["actions"]["patterns"];

    assert_eq!(patterns[0].as_str(), Some("*"));
}

#[test]
fn npm_groups_patch_and_minor_updates_by_kind_and_leaves_each_major_alone() {
    let updates = updates();

    let groups = update(&updates, "npm")["groups"]
        .as_mapping()
        .expect("the groups");

    let names: Vec<&str> = groups.keys().map(|k| k.as_str().expect("a name")).collect();
    assert_eq!(names, ["dev-minor-patch", "prod-minor-patch"]);
    for (name, kind) in [
        ("dev-minor-patch", "development"),
        ("prod-minor-patch", "production"),
    ] {
        let group = &groups[name];
        assert_eq!(group["dependency-type"].as_str(), Some(kind));
        let types: Vec<&str> = group["update-types"]
            .as_sequence()
            .expect("the update types")
            .iter()
            .map(|t| t.as_str().expect("a type"))
            .collect();
        assert_eq!(types, ["minor", "patch"], "{name}");
    }
}

#[test]
fn osv_scanner_is_the_one_of_python_with_the_lockfile_of_pnpm() {
    let node = workflow("ci-node", OSV);
    let python = osv_scanner_for("pnpm-lock.yaml");

    for key in ["on", "concurrency", "permissions", "jobs"] {
        assert_eq!(node[key], python[key], "{key}");
    }
    let args = node["jobs"]["osv-scanner"]["with"]["scan-args"].as_str();
    assert_eq!(args.map(str::trim), Some("--lockfile=pnpm-lock.yaml"));
}
