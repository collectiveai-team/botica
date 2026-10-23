//! `Recipes::tool_conflicts`: the packages that two recipes of a chain ask for with different
//! requirements, which `recipe-conflict` reports before `botica recipe install` fails on them.

mod common;

use botica_catalog::{RecipeItem, Recipes, ToolConflict};
use common::steps::{project, recipe};

fn tools(packages: &str) -> String {
    format!("[[tools]]\nmanager = \"uv\"\npackages = [{packages}]\n")
}

fn item(requirement: &str, recipe: &str) -> RecipeItem {
    RecipeItem {
        id: requirement.to_owned(),
        from: recipe.to_owned(),
    }
}

fn loaded() -> Recipes {
    project(vec![
        recipe("base", &tools("\"ruff\", \"pyrefly\"")),
        recipe(
            "team",
            &format!("extends = [\"base\"]\n\n{}", tools("\"ruff>=0.15\"")),
        ),
        recipe("async", &tools("\"blockbuster>=1.5,<1.6\", \"Pyrefly\"")),
        recipe("memory", &tools("\"blockbuster\"")),
    ])
    .expect("loads")
}

#[test]
fn a_recipe_that_asks_for_a_package_of_what_it_extends_with_another_requirement_conflicts() {
    let conflicts = loaded().tool_conflicts(&["team"]).expect("resolves");

    assert_eq!(
        conflicts,
        [ToolConflict {
            manager: "uv".to_owned(),
            group: "dev".to_owned(),
            package: "ruff".to_owned(),
            first: item("ruff", "base"),
            then: item("ruff>=0.15", "team"),
        }]
    );
    assert_eq!(
        conflicts[0].to_string(),
        "`team` asks for `ruff>=0.15` and `base` for `ruff`: a package has one requirement \
         across the recipes"
    );
}

#[test]
fn every_conflict_of_the_names_is_listed_in_the_order_of_the_chain() {
    let conflicts = loaded()
        .tool_conflicts(&["async", "team", "memory"])
        .expect("resolves");

    let pairs: Vec<(String, String, String)> = conflicts
        .iter()
        .map(|conflict| {
            (
                conflict.package.clone(),
                conflict.first.from.clone(),
                conflict.then.from.clone(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [
            ("ruff".to_owned(), "base".to_owned(), "team".to_owned()),
            (
                "blockbuster".to_owned(),
                "async".to_owned(),
                "memory".to_owned()
            ),
        ]
    );
}

#[test]
fn the_same_requirement_written_another_way_is_no_conflict() {
    let conflicts = loaded()
        .tool_conflicts(&["async", "base"])
        .expect("resolves");

    assert!(conflicts.is_empty(), "{conflicts:?}");
}

#[test]
fn no_name_has_no_conflict_and_a_name_that_is_not_a_recipe_fails() {
    let recipes = loaded();

    assert!(recipes
        .tool_conflicts::<&str>(&[])
        .expect("resolves")
        .is_empty());
    assert_eq!(
        recipes.tool_conflicts(&["nope"]),
        Err("no recipe `nope` (`botica recipe list` lists them)".to_owned())
    );
}

#[test]
fn resolve_many_fails_with_the_first_conflict() {
    let error = loaded()
        .resolve_many(&["team", "async", "memory"])
        .expect_err("does not resolve");

    assert_eq!(
        error,
        "`team` asks for `ruff>=0.15` and `base` for `ruff`: a package has one requirement \
         across the recipes"
    );
}
