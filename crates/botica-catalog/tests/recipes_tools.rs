//! The `tools` step of a recipe: packages that a package manager adds to the project (ADR 0006).

mod common;

use common::steps::{project, recipe};

const STEP: &str = "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"pyrefly\"]\n\n\
                    [[tools]]\nmanager = \"uv\"\ngroup = \"docs\"\npackages = [\"zensical\"]\n";

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}

#[test]
fn loads_the_tools_of_a_recipe_in_order_with_dev_as_the_default_group() {
    let loaded = project(vec![recipe("base", STEP)]).expect("loads");

    let base = loaded.recipe("base").expect("the recipe");
    assert_eq!(base.tools.len(), 2);
    assert_eq!(base.tools[0].manager, "uv");
    assert_eq!(base.tools[0].group, None);
    assert_eq!(base.tools[0].packages, ["ruff", "pyrefly"]);
    assert_eq!(base.tools[1].group.as_deref(), Some("docs"));
}

#[test]
fn a_recipe_without_tools_has_none() {
    let loaded = project(vec![recipe("bare", "rules = [\"a\"]\n")]).expect("loads");

    assert!(loaded.recipe("bare").expect("the recipe").tools.is_empty());
}

#[test]
fn an_unknown_key_is_rejected_and_the_manager_and_packages_are_needed() {
    let unknown = rejected("[[tools]]\nmanager = \"uv\"\npackages = [\"a\"]\nversion = \"1\"\n");
    assert!(unknown.contains("unknown field"), "{unknown}");

    for toml in [
        "[[tools]]\nmanager = \"uv\"\n",
        "[[tools]]\npackages = [\"a\"]\n",
    ] {
        let message = rejected(toml);
        assert!(message.contains("missing field"), "{message}");
    }
}

#[test]
fn only_a_known_manager_is_accepted() {
    let message = rejected("[[tools]]\nmanager = \"pip\"\npackages = [\"a\"]\n");

    assert!(
        message.contains("the `tools` manager `pip` is not one of: uv"),
        "{message}"
    );
}

#[test]
fn a_step_needs_packages_that_are_requirements() {
    let none = rejected("[[tools]]\nmanager = \"uv\"\npackages = []\n");
    assert!(none.contains("a `tools` step has no packages"), "{none}");

    for name in [
        "",
        " ",
        " a",
        "a ",
        "a b",
        "-a",
        "a/b",
        ">=1",
        "a @ https://example.com/a.whl",
        "a>=1 ; extra == '@'",
    ] {
        let message = rejected(&format!(
            "[[tools]]\nmanager = \"uv\"\npackages = [\"{name}\"]\n"
        ));

        assert!(
            message.contains(&format!(
                "the `tools` package `{name}` is not a requirement"
            )),
            "{name:?}: {message}"
        );
    }
    for name in [
        "a",
        "9lives",
        "Zope.Interface",
        "a_b-c.d9",
        "a>=1",
        "blockbuster>=1.5,<1.6",
        "a[x]==1",
        "pytest-memray ; sys_platform != 'win32'",
    ] {
        project(vec![recipe(
            "x",
            &format!("[[tools]]\nmanager = \"uv\"\npackages = [\"{name}\"]\n"),
        )])
        .expect("a package name");
    }
}

#[test]
fn a_group_is_a_name() {
    for group in ["", "a b", "--dev"] {
        let message = rejected(&format!(
            "[[tools]]\nmanager = \"uv\"\ngroup = \"{group}\"\npackages = [\"a\"]\n"
        ));

        assert!(
            message.contains(&format!("the `tools` group `{group}` is not a name")),
            "{group:?}: {message}"
        );
    }
}

#[test]
fn a_package_is_listed_once_in_a_recipe_whatever_the_spelling() {
    let message = rejected(
        "[[tools]]\nmanager = \"uv\"\npackages = [\"My_Pkg\"]\n\n\
         [[tools]]\nmanager = \"uv\"\ngroup = \"x\"\npackages = [\"my-pkg\"]\n",
    );

    assert!(
        message.contains("`tools` lists `my-pkg` twice"),
        "{message}"
    );
}

#[test]
fn packages_are_the_same_when_only_the_separators_and_the_case_differ() {
    for (first, second) in [("My.Pkg", "my-pkg"), ("a__b", "a-b"), ("a-.b", "a_b")] {
        let message = rejected(&format!(
            "[[tools]]\nmanager = \"uv\"\npackages = [\"{first}\", \"{second}\"]\n"
        ));

        assert!(
            message.contains(&format!("`tools` lists `{second}` twice")),
            "{first} {second}: {message}"
        );
    }
}

fn resolved(loaded: &botica_catalog::Recipes, id: &str) -> Vec<String> {
    loaded
        .resolve(id)
        .expect("resolves")
        .tools
        .iter()
        .map(|tool| {
            format!(
                "{} {} {} from {}",
                tool.manager, tool.group, tool.package, tool.from
            )
        })
        .collect()
}

#[test]
fn a_recipe_resolves_the_tools_of_what_it_extends_first_one_for_each_package() {
    let loaded = project(vec![
        recipe("base", STEP),
        recipe(
            "child",
            "extends = [\"base\"]\n\n[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"pytest\"]\n",
        ),
    ])
    .expect("loads");

    assert_eq!(
        resolved(&loaded, "child"),
        [
            "uv dev ruff from base",
            "uv dev pyrefly from base",
            "uv docs zensical from base",
            "uv dev pytest from child",
        ]
    );
}

#[test]
fn the_same_package_in_another_group_is_another_entry() {
    let loaded = project(vec![
        recipe("base", STEP),
        recipe(
            "other",
            "extends = [\"base\"]\n\n[[tools]]\nmanager = \"uv\"\ngroup = \"docs\"\npackages = [\"ruff\"]\n",
        ),
    ])
    .expect("loads");

    let tools = resolved(&loaded, "other");

    assert_eq!(tools.len(), 4);
    assert_eq!(tools[3], "uv docs ruff from other");
}

#[test]
fn a_package_is_listed_once_in_a_recipe_whatever_its_versions() {
    let message = rejected(
        "[[tools]]\nmanager = \"uv\"\npackages = [\"blockbuster>=1.5,<1.6\", \"blockbuster\"]\n",
    );

    assert!(
        message.contains("`tools` lists `blockbuster` twice"),
        "{message}"
    );
}

#[test]
fn a_package_that_two_recipes_ask_for_with_the_same_requirement_is_listed_once() {
    let loaded = project(vec![
        recipe(
            "base",
            "[[tools]]\nmanager = \"uv\"\npackages = [\"blockbuster>=1.5,<1.6\"]\n",
        ),
        recipe(
            "child",
            "extends = [\"base\"]\n\n[[tools]]\nmanager = \"uv\"\npackages = [\"Blockbuster >= 1.5, < 1.6\"]\n",
        ),
    ])
    .expect("loads");

    assert_eq!(
        resolved(&loaded, "child"),
        ["uv dev blockbuster>=1.5,<1.6 from base"]
    );
}

#[test]
fn a_package_that_two_recipes_ask_for_with_different_requirements_does_not_resolve() {
    let loaded = project(vec![
        recipe(
            "base",
            "[[tools]]\nmanager = \"uv\"\npackages = [\"blockbuster>=1.5,<1.6\"]\n",
        ),
        recipe(
            "child",
            "extends = [\"base\"]\n\n[[tools]]\nmanager = \"uv\"\npackages = [\"blockbuster\"]\n",
        ),
    ])
    .expect("loads");

    let error = loaded.resolve("child").expect_err("does not resolve");

    assert_eq!(
        error,
        "`child` asks for `blockbuster` and `base` for `blockbuster>=1.5,<1.6`: a package has one \
         requirement across the recipes"
    );
}

#[test]
fn two_names_that_ask_for_a_package_with_different_requirements_do_not_resolve() {
    let loaded = project(vec![
        recipe(
            "one",
            "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff>=0.14\"]\n",
        ),
        recipe(
            "two",
            "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
        ),
    ])
    .expect("loads");

    assert!(loaded.resolve_many(&["one", "two"]).is_err());
}

#[test]
fn each_built_in_recipe_resolves() {
    let recipes = botica_catalog::Recipes::embedded().expect("the built-in recipes load");

    for recipe in recipes.recipes() {
        assert!(
            recipes.resolve(&recipe.id).is_ok(),
            "{}: {:?}",
            recipe.id,
            recipes.resolve(&recipe.id).err()
        );
    }
}
