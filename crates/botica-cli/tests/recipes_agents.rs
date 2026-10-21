//! `botica recipe install` installs the skills for the agents that the named recipes list, and
//! for `opencode` when none does. The tests put a `npx` of their own first on the `PATH`.
#![cfg(unix)]

mod common;

use common::recipes::{add_recipe, npx_calls, project, recipe, with_npx, NPX_INSTALLS};
use common::refs::ok;

const SKILL: &str = "[[skills]]\nsource = \"acme/skills\"\nnames = [\"tdd\"]\n";

fn calls_of(name: &str, recipes: &[(&str, &str)], install: &[&str]) -> Vec<String> {
    let dir = project(name, None);
    for (id, toml) in recipes {
        add_recipe(&dir, id, toml);
    }
    let mut args = vec!["install"];
    args.extend_from_slice(install);
    ok(&with_npx(&dir, NPX_INSTALLS, &args));
    npx_calls(&dir)
}

#[test]
fn without_agents_the_skills_are_installed_for_opencode() {
    let calls = calls_of("agents-none", &[("mine", SKILL)], &["mine"]);

    assert_eq!(
        calls,
        ["skills add acme/skills --agent opencode --yes --json --skill tdd"]
    );
}

#[test]
fn the_agents_of_a_recipe_replace_the_default() {
    let toml = format!("agents = [\"claude-code\", \"codex\"]\n{SKILL}");

    let calls = calls_of("agents-own", &[("mine", &toml)], &["mine"]);

    assert_eq!(
        calls,
        ["skills add acme/skills --agent claude-code codex --yes --json --skill tdd"]
    );
}

#[test]
fn the_agents_of_what_a_recipe_extends_and_of_every_named_recipe_are_joined_in_order() {
    let calls = calls_of(
        "agents-joined",
        &[
            ("parent", "agents = [\"opencode\"]\n"),
            (
                "child",
                "extends = [\"parent\"]\nagents = [\"claude-code\"]\n",
            ),
            ("other", "agents = [\"claude-code\", \"codex\"]\n"),
            ("skills", SKILL),
        ],
        &["child", "other", "skills"],
    );

    assert_eq!(
        calls,
        ["skills add acme/skills --agent opencode claude-code codex --yes --json --skill tdd"]
    );
}

#[test]
fn every_source_is_installed_for_the_same_agents() {
    let toml = "agents = [\"codex\"]\n[[skills]]\nsource = \"a/one\"\nnames = [\"x\"]\n\
                [[skills]]\nsource = \"b/two\"\nnames = [\"y\"]\n";

    let calls = calls_of("agents-sources", &[("mine", toml)], &["mine"]);

    assert_eq!(
        calls,
        [
            "skills add a/one --agent codex --yes --json --skill x",
            "skills add b/two --agent codex --yes --json --skill y"
        ]
    );
}

#[test]
fn show_and_list_say_which_agents_a_recipe_has() {
    let dir = project("agents-show", None);
    add_recipe(&dir, "parent", "agents = [\"opencode\"]\n");
    add_recipe(
        &dir,
        "child",
        "extends = [\"parent\"]\nagents = [\"claude-code\"]\n",
    );

    let shown = ok(&recipe(&dir, &["show", "child"]));
    let listed = ok(&recipe(&dir, &["list"]));

    assert!(
        shown.contains("\n## Agents\n\n- opencode (from parent)\n- claude-code (from child)\n"),
        "{shown}"
    );
    assert!(listed.contains("child\n  extends: parent\n"), "{listed}");
    assert!(listed.contains("  agents: 1\n"), "{listed}");
}
