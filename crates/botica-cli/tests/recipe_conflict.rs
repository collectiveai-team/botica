//! `recipe-conflict`: `botica lint` and `botica recipe check` report two recipes that the project
//! uses and that ask for one package with different requirements.

mod common;

use common::recipes::{add_recipe, project, recipe};
use common::refs::failed;
use common::{botica, stdout};

const SELECTED: &str = "select = [\"recipe-conflict\"]\npreview = true\n";

const TEAM: &str = "extends = [\"base-python\"]\n\n[[tools]]\nmanager = \"uv\"\n\
                    packages = [\"ruff>=0.15\"]\n";

#[test]
fn lint_reports_a_recipe_of_the_project_that_asks_for_a_package_of_what_it_extends_again() {
    let dir = project("conflict-extends", Some(SELECTED));
    add_recipe(&dir, "team", TEAM);

    let output = botica(&dir, &["--scope", "project"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains(
            "recipes/team.toml:5:13: error [recipe-conflict] The recipes `team` and `base-python` \
             ask for the package `ruff` as `ruff>=0.15` and as `ruff`"
        ),
        "{text}"
    );
}

#[test]
fn lint_resolves_the_installed_recipes_with_the_ones_of_the_project() {
    let config = format!("{SELECTED}\n[recipes]\ninstalled = [\"base-python\"]\n");
    let dir = project("conflict-installed", Some(&config));
    add_recipe(
        &dir,
        "docs",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"pytest>=9\"]\n",
    );

    let text = stdout(&botica(&dir, &["--scope", "project"]));

    assert!(
        text.contains(
            "recipes/docs.toml:3:13: error [recipe-conflict] The recipes `docs` and \
                       `base-python` ask for the package `pytest`"
        ),
        "{text}"
    );
}

#[test]
fn an_installed_recipe_that_does_not_exist_is_a_finding_about_the_configuration() {
    let config = format!("{SELECTED}\n[recipes]\ninstalled = [\"nope\"]\n");
    let dir = project("conflict-unknown", Some(&config));

    let text = stdout(&botica(&dir, &["--scope", "project"]));

    assert!(
        text.contains(
            "botica.toml:1:1: error [recipe-conflict] botica cannot resolve the recipes of the \
             project, so it does not look for conflicts between them: no recipe `nope`"
        ),
        "{text}"
    );
}

#[test]
fn the_configuration_switches_the_rule_off_and_a_project_without_recipes_has_nothing() {
    let dir = project(
        "conflict-off",
        Some(&format!("{SELECTED}ignore = [\"recipe-conflict\"]\n")),
    );
    add_recipe(&dir, "team", TEAM);
    assert!(!stdout(&botica(&dir, &["--scope", "project"])).contains("recipe-conflict"));

    let dir = project("conflict-none", Some(SELECTED));
    assert!(!stdout(&botica(&dir, &["--scope", "project"])).contains("recipe-conflict"));
}

#[test]
fn check_prints_a_conflict_of_the_names_with_the_recipes_of_the_project() {
    let dir = project("conflict-check", None);
    add_recipe(&dir, "team", TEAM);

    let text = failed(&recipe(&dir, &["check", "base-python"]), 1);

    assert!(
        text.contains(
            "conflict ruff: differs: The recipes `team` and `base-python` ask for the package \
             `ruff` as `ruff>=0.15` and as `ruff`"
        ),
        "{text}"
    );
}
