//! Several recipes resolved as one: what `botica recipe install` and `check` do with the names
//! they are given (ADR 0006).

mod common;

use botica_catalog::Recipes;
use common::steps::{project, recipe, STEPS};

fn loaded() -> Recipes {
    project(vec![
        recipe("base", &format!("rules = [\"one\"]\n{STEPS}")),
        recipe(
            "python",
            "extends = [\"base\"]\nrules = [\"one\", \"two\"]\njudgment = [\"j\"]\nexports = [\"agents-md\"]\n\n\
             [[files]]\npath = \"pyproject.toml\"\ntemplate = \"files/pyproject.toml\"\n",
        ),
        recipe(
            "agent",
            "extends = [\"base\"]\nrules = [\"three\"]\njudgment = [\"j\", \"k\"]\nagents = [\"opencode\"]\n\n\
             [[files]]\npath = \"opencode.jsonc\"\ntemplate = \"files/opencode.jsonc\"\n\n\
             [[files]]\npath = \"pyproject.toml\"\ntemplate = \"files/pyproject.toml\"\n",
        ),
        recipe(
            "fork",
            "extends = [\"base\"]\n\n[[skills]]\nsource = \"fork/skills\"\nnames = [\"handoff\"]\n",
        ),
    ])
    .expect("loads")
}

#[test]
fn the_recipes_are_one_chain_with_each_recipe_once_in_the_order_they_are_named() {
    let recipes = loaded();

    let resolved = recipes
        .resolve_many(&["python", "agent"])
        .expect("resolves");

    assert_eq!(resolved.chain, ["base", "python", "agent"]);
    assert_eq!(resolved.id, "python, agent");
}

#[test]
fn a_recipe_that_extends_one_already_named_does_not_bring_it_again() {
    let recipes = loaded();

    let resolved = recipes.resolve_many(&["base", "python"]).expect("resolves");

    assert_eq!(resolved.chain, ["base", "python"]);
    assert_eq!(resolved.rules.len(), 2);
}

#[test]
fn a_rule_a_judgment_rule_or_an_export_that_two_recipes_list_stays_where_it_first_is() {
    let recipes = loaded();

    let resolved = recipes
        .resolve_many(&["python", "agent"])
        .expect("resolves");

    let pairs = |items: &[botica_catalog::RecipeItem]| -> Vec<String> {
        items
            .iter()
            .map(|item| format!("{} from {}", item.id, item.from))
            .collect()
    };
    assert_eq!(
        pairs(&resolved.rules),
        ["one from base", "two from python", "three from agent"]
    );
    assert_eq!(pairs(&resolved.judgment), ["j from python", "k from agent"]);
    assert_eq!(pairs(&resolved.exports), ["agents-md from python"]);
    assert_eq!(pairs(&resolved.agents), ["opencode from agent"]);
}

#[test]
fn a_step_that_two_named_recipes_list_the_same_stays_once_where_it_first_is() {
    let recipes = loaded();

    let resolved = recipes
        .resolve_many(&["python", "agent"])
        .expect("resolves");

    let files: Vec<String> = resolved
        .files
        .iter()
        .map(|file| format!("{} from {}", file.path, file.from))
        .collect();
    assert_eq!(
        files,
        [
            "prek.toml from base",
            "pyproject.toml from python",
            "opencode.jsonc from agent"
        ]
    );
}

#[test]
fn a_later_name_takes_over_a_skill_from_another_source_and_warns() {
    let recipes = loaded();

    let resolved = recipes.resolve_many(&["base", "fork"]).expect("resolves");

    let handoff = resolved
        .skills
        .iter()
        .find(|skill| skill.name == "handoff")
        .expect("the skill");
    assert_eq!(handoff.source, "fork/skills");
    assert_eq!(
        resolved.warnings,
        ["the skill `handoff` comes from `fork/skills` (fork), which replaces `collectiveai-team/botica` (base)"]
    );
}

#[test]
fn a_single_name_resolves_as_resolve_does() {
    let recipes = loaded();

    let many = recipes.resolve_many(&["python"]).expect("resolves");

    assert_eq!(many, recipes.resolve("python").expect("resolves"));
}

#[test]
fn a_name_that_is_not_a_recipe_fails_whatever_its_place() {
    let recipes = loaded();

    for names in [["nope", "base"], ["base", "nope"]] {
        let error = recipes.resolve_many(&names).expect_err("fails");

        assert!(error.starts_with("no recipe `nope`"), "{error}");
    }
}

#[test]
fn no_name_is_an_error() {
    let recipes = loaded();

    let names: [&str; 0] = [];
    let error = recipes.resolve_many(&names).expect_err("fails");

    assert!(error.contains("at least one"), "{error}");
}
