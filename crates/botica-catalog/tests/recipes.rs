//! Recipes: a file `<id>.toml` that lists rules and exports, and extends other recipes (ADR 0006).

mod common;

use botica_catalog::{CatalogError, Origin, Recipes, SourceFile};
use common::file;

const PYTHON: &str = r#"description = "Python projects"
extends = ["generic"]
rules = ["no-utils", "log-no-print"]
judgment = ["test-through-interface"]
exports = ["agents-md", "abide"]
"#;

fn recipe(id: &str, toml: &str) -> SourceFile {
    file(&format!("{id}.toml"), toml)
}

fn project(files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    Recipes::from_files(&files)
}

fn builtin(files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    Recipes::from_builtin_files(&files)
}

fn over(
    builtin: &Recipes,
    files: Vec<SourceFile>,
    replace: &[&str],
) -> Result<Recipes, CatalogError> {
    let replace: Vec<String> = replace.iter().map(|id| (*id).to_owned()).collect();
    Recipes::over(builtin, &files, &replace)
}

fn rejected(result: Result<Recipes, CatalogError>) -> String {
    result.expect_err("rejected").to_string()
}

fn ids(recipes: &Recipes) -> Vec<&str> {
    recipes.recipes().iter().map(|r| r.id.as_str()).collect()
}

#[test]
fn loads_a_recipe_with_its_lists_from_the_name_of_its_file() {
    let loaded = project(vec![recipe("python", PYTHON)]).expect("loads");

    let python = loaded.recipe("python").expect("the recipe");
    assert_eq!(python.id, "python");
    assert_eq!(python.description.as_deref(), Some("Python projects"));
    assert_eq!(python.extends, ["generic"]);
    assert_eq!(python.rules, ["no-utils", "log-no-print"]);
    assert_eq!(python.judgment, ["test-through-interface"]);
    assert_eq!(python.exports, ["agents-md", "abide"]);
    assert_eq!(python.origin, Origin::Project);
    assert!(loaded.recipe("nope").is_none());
}

#[test]
fn every_list_is_optional_and_a_file_may_be_empty() {
    let loaded = project(vec![recipe("bare", "")]).expect("loads");

    let bare = loaded.recipe("bare").expect("the recipe");
    assert_eq!(bare.description, None);
    assert!(bare.extends.is_empty() && bare.rules.is_empty());
    assert!(bare.judgment.is_empty() && bare.exports.is_empty());
}

#[test]
fn the_recipes_come_in_the_order_of_their_ids() {
    let loaded = project(vec![recipe("zeta", ""), recipe("alpha", "")]).expect("loads");

    assert_eq!(ids(&loaded), ["alpha", "zeta"]);
}

#[test]
fn a_recipe_knows_whether_it_is_built_in_or_of_the_project() {
    let built_in = builtin(vec![recipe("generic", "")]).expect("loads");

    assert_eq!(
        built_in.recipe("generic").expect("recipe").origin,
        Origin::BuiltIn
    );
}

#[test]
fn an_unknown_key_is_rejected_and_names_the_file() {
    let message = rejected(project(vec![recipe("python", "rule = [\"a\"]\n")]));

    assert!(message.contains("python.toml"), "{message}");
    assert!(message.contains("rule"), "{message}");
}

#[test]
fn the_id_is_a_kebab_case_file_name() {
    let message = rejected(project(vec![recipe("Python", "")]));

    assert!(message.contains("Python.toml"), "{message}");
    assert!(message.contains("kebab-case"), "{message}");
}

#[test]
fn a_file_that_is_not_a_recipe_is_rejected() {
    for path in ["python/recipe.toml", "python.md", "README.md"] {
        let message = rejected(project(vec![file(path, "")]));

        assert!(message.contains(path), "{path}: {message}");
        assert!(
            message.contains("expected <recipe>.toml"),
            "{path}: {message}"
        );
    }
}

#[test]
fn an_export_is_one_of_the_two_that_exist() {
    let message = rejected(project(vec![recipe("a", "exports = [\"pdf\"]\n")]));

    assert!(message.contains("a.toml"), "{message}");
    assert!(message.contains("unknown export `pdf`"), "{message}");
    assert!(message.contains("agents-md, abide"), "{message}");
}

#[test]
fn a_list_names_each_entry_once() {
    for (key, toml) in [
        ("rules", "rules = [\"a\", \"b\", \"a\"]\n"),
        ("judgment", "judgment = [\"a\", \"a\"]\n"),
        ("exports", "exports = [\"abide\", \"abide\"]\n"),
        ("extends", "extends = [\"a\", \"a\"]\n"),
    ] {
        let message = rejected(project(vec![recipe("x", toml)]));

        assert!(
            message.contains(&format!("`{key}` lists `")),
            "{key}: {message}"
        );
        assert!(message.contains("twice"), "{key}: {message}");
    }
}

#[test]
fn an_entry_that_is_blank_is_rejected() {
    let message = rejected(project(vec![recipe("x", "rules = [\"a\", \" \"]\n")]));

    assert!(message.contains("`rules` has an empty entry"), "{message}");
}

#[test]
fn the_problems_of_every_file_are_reported_together() {
    let message = rejected(project(vec![
        recipe("one", "exports = [\"pdf\"]\n"),
        recipe("two", "exports = [\"doc\"]\n"),
    ]));

    assert!(
        message.contains("one.toml") && message.contains("two.toml"),
        "{message}"
    );
}

#[test]
fn a_built_in_recipe_extends_recipes_that_exist() {
    let message = rejected(builtin(vec![recipe("python", "extends = [\"generic\"]\n")]));

    assert!(message.contains("python.toml"), "{message}");
    assert!(
        message.contains("extends `generic`, which is not a recipe"),
        "{message}"
    );
}

#[test]
fn recipes_that_extend_each_other_are_rejected() {
    for files in [
        vec![recipe("a", "extends = [\"a\"]\n")],
        vec![
            recipe("a", "extends = [\"b\"]\n"),
            recipe("b", "extends = [\"a\"]\n"),
        ],
        vec![
            recipe("a", "extends = [\"b\"]\n"),
            recipe("b", "extends = [\"c\"]\n"),
            recipe("c", "extends = [\"a\"]\n"),
        ],
    ] {
        let message = rejected(builtin(files));

        assert!(message.contains("extends itself"), "{message}");
    }
}

#[test]
fn a_recipe_that_is_extended_twice_is_not_a_cycle() {
    builtin(vec![
        recipe("base", ""),
        recipe("left", "extends = [\"base\"]\n"),
        recipe("right", "extends = [\"base\"]\n"),
        recipe("top", "extends = [\"left\", \"right\"]\n"),
    ])
    .expect("a diamond is not a cycle");
}

#[test]
fn a_recipe_of_the_project_may_extend_a_built_in_one() {
    let built_in = builtin(vec![recipe("generic", "rules = [\"a\"]\n")]).expect("loads");

    let all = over(
        &built_in,
        vec![recipe("mine", "extends = [\"generic\"]\n")],
        &[],
    )
    .expect("loads");

    assert_eq!(ids(&all), ["generic", "mine"]);
    assert_eq!(all.recipe("mine").expect("recipe").origin, Origin::Project);
}

#[test]
fn a_recipe_of_the_project_extends_recipes_that_exist() {
    let built_in = builtin(vec![recipe("generic", "")]).expect("loads");

    let message = rejected(over(
        &built_in,
        vec![recipe("mine", "extends = [\"nope\"]\n")],
        &[],
    ));

    assert!(message.contains("mine.toml"), "{message}");
    assert!(
        message.contains("extends `nope`, which is not a recipe"),
        "{message}"
    );
}

#[test]
fn a_recipe_of_the_project_with_a_built_in_id_is_an_error_unless_replaced() {
    let built_in = builtin(vec![recipe("generic", "rules = [\"a\"]\n")]).expect("loads");

    let message = rejected(over(&built_in, vec![recipe("generic", "")], &[]));

    assert!(message.contains("generic.toml"), "{message}");
    assert!(message.contains("built-in recipe"), "{message}");
    assert!(message.contains("[recipes] replace"), "{message}");
}

#[test]
fn a_replaced_recipe_takes_the_place_of_the_built_in_one() {
    let built_in = builtin(vec![recipe("generic", "rules = [\"a\"]\n")]).expect("loads");

    let all = over(
        &built_in,
        vec![recipe("generic", "rules = [\"b\"]\n")],
        &["generic"],
    )
    .expect("loads");

    let generic = all.recipe("generic").expect("recipe");
    assert_eq!(generic.rules, ["b"]);
    assert_eq!(generic.origin, Origin::Project);
    assert_eq!(ids(&all), ["generic"]);
}

#[test]
fn replace_names_a_built_in_recipe_that_the_project_has() {
    let built_in = builtin(vec![recipe("generic", "")]).expect("loads");

    let unknown = rejected(over(&built_in, vec![], &["nope"]));
    assert!(unknown.contains("[recipes] replace"), "{unknown}");
    assert!(
        unknown.contains("`nope` is not a built-in recipe"),
        "{unknown}"
    );

    let absent = rejected(over(&built_in, vec![], &["generic"]));
    assert!(
        absent.contains("`generic` is a built-in recipe"),
        "{absent}"
    );
    assert!(
        absent.contains("`recipes/` has no recipe with that id"),
        "{absent}"
    );
}

#[test]
fn the_recipes_of_both_origins_are_in_the_order_of_their_ids() {
    let built_in = builtin(vec![recipe("generic", ""), recipe("python", "")]).expect("loads");

    let all = over(&built_in, vec![recipe("mine", ""), recipe("zed", "")], &[]).expect("loads");

    assert_eq!(ids(&all), ["generic", "mine", "python", "zed"]);
}

fn names(items: &[botica_catalog::RecipeItem]) -> Vec<(&str, &str)> {
    items
        .iter()
        .map(|item| (item.id.as_str(), item.from.as_str()))
        .collect()
}

#[test]
fn resolving_a_recipe_puts_what_it_extends_first_and_says_where_each_entry_comes_from() {
    let all = builtin(vec![
        recipe("generic", "rules = [\"file-size-guard\"]\nexports = [\"agents-md\"]\n"),
        recipe(
            "python",
            "extends = [\"generic\"]\nrules = [\"no-utils\"]\njudgment = [\"j\"]\nexports = [\"abide\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = all.resolve("python").expect("resolves");

    assert_eq!(resolved.id, "python");
    assert_eq!(resolved.chain, ["generic", "python"]);
    assert_eq!(
        names(&resolved.rules),
        [("file-size-guard", "generic"), ("no-utils", "python")]
    );
    assert_eq!(names(&resolved.judgment), [("j", "python")]);
    assert_eq!(
        names(&resolved.exports),
        [("agents-md", "generic"), ("abide", "python")]
    );
}

#[test]
fn an_entry_that_two_recipes_list_appears_once_where_it_first_is() {
    let all = builtin(vec![
        recipe("base", "rules = [\"a\", \"b\"]\n"),
        recipe("left", "extends = [\"base\"]\nrules = [\"b\", \"c\"]\n"),
        recipe("right", "extends = [\"base\"]\nrules = [\"d\"]\n"),
        recipe(
            "top",
            "extends = [\"left\", \"right\"]\nrules = [\"a\", \"e\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = all.resolve("top").expect("resolves");

    assert_eq!(resolved.chain, ["base", "left", "right", "top"]);
    assert_eq!(
        names(&resolved.rules),
        [
            ("a", "base"),
            ("b", "base"),
            ("c", "left"),
            ("d", "right"),
            ("e", "top")
        ]
    );
}

#[test]
fn resolving_a_recipe_that_does_not_exist_says_so() {
    let all = builtin(vec![recipe("generic", "")]).expect("loads");

    let message = all.resolve("nope").expect_err("unknown");

    assert!(message.contains("no recipe `nope`"), "{message}");
}

#[test]
fn a_cycle_is_reported_once_and_names_the_recipes_in_it() {
    let message = rejected(builtin(vec![
        recipe("a", "extends = [\"b\"]\n"),
        recipe("b", "extends = [\"a\"]\n"),
    ]));

    assert_eq!(message.lines().count(), 1, "{message}");
    assert!(message.contains("extends itself: a -> b -> a"), "{message}");
}

#[test]
fn resolving_a_recipe_of_a_project_that_extends_itself_fails() {
    let loaded = project(vec![
        recipe("a", "extends = [\"b\"]\n"),
        recipe("b", "extends = [\"a\"]\n"),
    ])
    .expect("a project is not checked for cycles until it joins the built-in ones");

    let message = loaded.resolve("a").expect_err("a cycle");

    assert!(message.contains("extends itself"), "{message}");
}

#[test]
fn resolving_a_recipe_that_extends_one_that_is_not_there_names_both() {
    let loaded = project(vec![recipe("a", "extends = [\"generic\"]\n")]).expect("loads");

    let message = loaded.resolve("a").expect_err("unknown parent");

    assert!(
        message.contains("`a` extends `generic`, which is not a recipe"),
        "{message}"
    );
}
