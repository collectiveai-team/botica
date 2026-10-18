//! The `agents` key of a recipe: the agents that `npx skills add` installs for (ADR 0006).

mod common;

use common::steps::{project, recipe};

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}

#[test]
fn loads_the_agents_of_a_recipe() {
    let loaded =
        project(vec![recipe("x", "agents = [\"claude-code\", \"codex\"]\n")]).expect("loads");

    assert_eq!(
        loaded.recipe("x").expect("the recipe").agents,
        ["claude-code", "codex"]
    );
}

#[test]
fn a_recipe_without_the_key_has_no_agents() {
    let loaded = project(vec![recipe("x", "rules = [\"a\"]\n")]).expect("loads");

    assert!(loaded.recipe("x").expect("the recipe").agents.is_empty());
}

#[test]
fn only_the_known_agents_are_accepted() {
    let message = rejected("agents = [\"emacs\"]\n");

    assert!(
        message.contains("unknown agent `emacs` (expected one of opencode, claude-code, codex)"),
        "{message}"
    );
}

#[test]
fn an_agent_is_listed_once_and_is_not_empty() {
    let twice = rejected("agents = [\"codex\", \"codex\"]\n");
    assert!(twice.contains("`agents` lists `codex` twice"), "{twice}");

    let empty = rejected("agents = [\"\"]\n");
    assert!(empty.contains("`agents` has an empty entry"), "{empty}");
}

#[test]
fn a_recipe_resolves_the_agents_of_what_it_extends_first_each_once() {
    let loaded = project(vec![
        recipe("base", "agents = [\"opencode\", \"codex\"]\n"),
        recipe(
            "child",
            "extends = [\"base\"]\nagents = [\"claude-code\", \"codex\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = loaded.resolve("child").expect("resolves");

    let agents: Vec<String> = resolved
        .agents
        .iter()
        .map(|item| format!("{} from {}", item.id, item.from))
        .collect();
    assert_eq!(
        agents,
        [
            "opencode from base",
            "codex from base",
            "claude-code from child"
        ]
    );
}
