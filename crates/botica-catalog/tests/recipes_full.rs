//! The recipes `full-*`: the `base-*` of a language and its `ci-*`, with no step of their own.

use botica_catalog::Recipes;

fn recipes() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

#[test]
fn a_full_recipe_extends_the_base_of_its_language_and_has_no_step_of_its_own() {
    let recipes = recipes();

    for recipe in recipes
        .recipes()
        .iter()
        .filter(|r| r.id.starts_with("full-"))
    {
        let language = &recipe.id["full-".len()..];
        assert_eq!(
            recipe.extends.first(),
            Some(&format!("base-{language}")),
            "{}",
            recipe.id
        );
        assert!(
            recipe.rules.is_empty()
                && recipe.judgment.is_empty()
                && recipe.exports.is_empty()
                && recipe.skills.is_empty()
                && recipe.files.is_empty()
                && recipe.sections.is_empty()
                && recipe.links.is_empty()
                && recipe.gitignore.is_empty()
                && recipe.tools.is_empty()
                && recipe.params.is_empty(),
            "{}",
            recipe.id
        );
    }
}

#[test]
fn full_python_resolves_to_what_base_python_and_ci_python_resolve_to() {
    let recipes = recipes();

    let full = recipes.resolve("full-python").expect("resolves");
    let both = recipes
        .resolve_many(&["base-python", "ci-python"])
        .expect("resolves");

    assert_eq!(
        full.chain,
        ["base", "base-python", "ci", "ci-python", "full-python"]
    );
    assert_eq!(full.rules, both.rules);
    assert_eq!(full.files, both.files);
    assert_eq!(full.sections, both.sections);
    assert_eq!(full.tools, both.tools);
    assert_eq!(full.skills, both.skills);
    assert_eq!(full.params, both.params);
}
