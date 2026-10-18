//! The recipes that the binary carries.

use botica_catalog::{Catalog, Kind, Origin, Recipes};

fn embedded() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

#[test]
fn the_binary_carries_the_recipes_base_and_base_python() {
    let recipes = embedded();

    let ids: Vec<&str> = recipes.recipes().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["base", "base-python"]);
    for recipe in recipes.recipes() {
        assert_eq!(recipe.origin, Origin::BuiltIn);
        assert!(
            recipe.description.is_some(),
            "{} has no description",
            recipe.id
        );
    }
    assert_eq!(
        recipes.recipe("base-python").expect("base-python").extends,
        ["base"]
    );
}

#[test]
fn every_rule_of_a_built_in_recipe_is_in_the_built_in_catalog() {
    let recipes = embedded();
    let catalog = Catalog::embedded().expect("the catalog loads");

    for recipe in recipes.recipes() {
        for id in recipe.rules.iter().chain(&recipe.judgment) {
            assert!(
                catalog.resolve(id).is_some(),
                "recipe {} lists `{id}`, which is not in the catalog",
                recipe.id
            );
        }
    }
}

#[test]
fn a_built_in_recipe_lists_judgment_rules_apart_from_the_others() {
    let recipes = embedded();
    let catalog = Catalog::embedded().expect("the catalog loads");
    let is_judgment = |id: &String| {
        catalog.resolve(id).is_some_and(|rule| {
            rule.implementations
                .iter()
                .any(|i| i.kind == Kind::Judgment)
        })
    };

    for recipe in recipes.recipes() {
        assert!(
            !recipe.rules.iter().any(is_judgment),
            "recipe {} lists a judgment rule under `rules`",
            recipe.id
        );
        assert!(
            recipe.judgment.iter().all(is_judgment),
            "recipe {} lists under `judgment` a rule that is not a judgment rule",
            recipe.id
        );
    }
}

#[test]
fn base_python_resolves_to_the_rules_of_base_first() {
    let resolved = embedded().resolve("base-python").expect("resolves");

    assert_eq!(resolved.chain, ["base", "base-python"]);
    let first: Vec<&str> = resolved
        .rules
        .iter()
        .take(3)
        .map(|i| i.id.as_str())
        .collect();
    assert_eq!(
        first,
        ["file-size-guard", "no-notebooks", "no-artifacts-in-package"]
    );
    assert_eq!(resolved.rules.len(), 15);
}
