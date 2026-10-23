//! The parameters that a recipe declares under `[params.<name>]` and its templates write as
//! `{{ param.<name> }}` (ADR 0006).

mod common;

use std::collections::BTreeMap;

use botica_catalog::{render, ParamKind, Recipes};
use common::file;
use common::steps::recipe;

const DAYS: &str = "[params.cooldown-days]\ndescription = \"Days a release waits\"\n\
                    kind = \"days\"\ndefault = 7\n";

fn loaded(files: Vec<botica_catalog::SourceFile>) -> Result<Recipes, String> {
    Recipes::from_files(&files).map_err(|error| error.to_string())
}

#[test]
fn a_recipe_declares_a_parameter_with_its_kind_and_its_default() {
    let recipes = loaded(vec![recipe("mine", DAYS)]).expect("loads");

    let spec = &recipes.recipe("mine").expect("mine").params["cooldown-days"];

    assert_eq!(spec.kind, ParamKind::Days);
    assert_eq!(spec.default, "7");
    assert!(recipes.declares("cooldown-days"));
    assert!(!recipes.declares("go-preference"));
}

#[test]
fn a_default_out_of_its_kind_or_its_choices_does_not_load() {
    let error =
        loaded(vec![
        recipe(
            "days",
            "[params.cooldown-days]\ndescription = \"d\"\nkind = \"days\"\ndefault = \"a week\"\n",
        ),
        recipe(
            "go",
            "[params.go-preference]\ndescription = \"d\"\ndefault = \"mine\"\n\
             choices = [\"system\", \"managed\"]\n",
        ),
        recipe("color", "[params.x]\ndescription = \"d\"\ndefault = \"\"\nkind = \"color\"\n"),
    ])
        .expect_err("fails");

    assert!(error.contains("color.toml"), "{error}");
    assert!(error.contains("days.toml"), "{error}");
    assert!(error.contains("is not a whole number of days"), "{error}");
    assert!(
        error.contains(
            "the default `mine` of the parameter `go-preference` is not one of its choices"
        ),
        "{error}"
    );
}

#[test]
fn two_recipes_of_a_chain_may_declare_a_parameter_the_same_and_not_differently() {
    let same = loaded(vec![
        recipe("a", DAYS),
        recipe("b", &format!("extends = [\"a\"]\n{DAYS}")),
    ])
    .expect("loads");
    let other = loaded(vec![
        recipe("a", DAYS),
        recipe(
            "b",
            "[params.cooldown-days]\ndescription = \"Days a release waits\"\nkind = \"days\"\n\
             default = 14\n",
        ),
    ])
    .expect("loads");

    assert_eq!(same.resolve("b").expect("resolves").params.len(), 1);
    let error = other.resolve_many(&["a", "b"]).expect_err("fails");
    assert_eq!(
        error,
        "`b` and `a` declare the parameter `cooldown-days` differently"
    );
}

#[test]
fn a_template_is_checked_against_the_parameters_of_the_whole_chain() {
    let recipes = loaded(vec![
        recipe("a", DAYS),
        recipe(
            "b",
            "[[files]]\npath = \"pnpm-workspace.yaml\"\ntemplate = \"files/pnpm.yaml\"\n",
        ),
        recipe(
            "c",
            "extends = [\"a\"]\n\n[[files]]\npath = \"pnpm-workspace.yaml\"\n\
             template = \"files/pnpm.yaml\"\n",
        ),
        file(
            "files/pnpm.yaml",
            "minimumReleaseAge: {{ param.cooldown-days.minutes }}\n",
        ),
    ])
    .expect("loads");

    assert!(recipes.resolve("b").is_err());
    assert!(recipes.resolve_many(&["a", "b"]).is_ok());
    let mut resolved = recipes.resolve("c").expect("resolves");
    recipes
        .set_params(
            &mut resolved,
            &BTreeMap::from([("cooldown-days".to_owned(), "14".to_owned())]),
            &BTreeMap::new(),
        )
        .expect("sets");
    let text = recipes
        .template("c", "files/pnpm.yaml")
        .expect("a template");
    assert_eq!(
        render(text, &resolved.params).expect("renders"),
        "minimumReleaseAge: 20160\n"
    );
}

#[test]
fn every_built_in_parameter_has_one_declaration_and_every_built_in_recipe_resolves() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let mut seen = BTreeMap::new();
    for recipe in recipes.recipes() {
        for (name, spec) in &recipe.params {
            let first = seen
                .entry(name.clone())
                .or_insert((spec, recipe.id.clone()));
            assert_eq!(
                first.0, spec,
                "`{}` and `{}` declare `{name}` differently",
                first.1, recipe.id
            );
        }
        recipes.resolve(&recipe.id).expect("resolves");
    }
    assert!(seen.contains_key("cooldown-days"), "{seen:?}");
}
