//! The steps of the recipes that a recipe extends, and how they resolve (ADR 0006).

mod common;

use botica_catalog::Recipes;
use common::steps::{project, recipe, STEPS};

fn resolved_names(recipes: &Recipes, id: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let resolved = recipes.resolve(id).expect("resolves");
    (
        resolved
            .skills
            .iter()
            .map(|s| format!("{}={} from {}", s.name, s.source, s.from))
            .collect(),
        resolved
            .files
            .iter()
            .map(|f| format!("{}={} from {}", f.path, f.template, f.from))
            .collect(),
        resolved
            .sections
            .iter()
            .map(|s| format!("{}#{}={} from {}", s.path, s.marker, s.template, s.from))
            .collect(),
    )
}

#[test]
fn a_recipe_resolves_the_steps_of_what_it_extends_first_with_the_recipe_of_each() {
    let loaded = project(vec![
        recipe("base", STEPS),
        recipe(
            "base-python",
            "extends = [\"base\"]\n\n\
             [[skills]]\nsource = \"a/b\"\nnames = [\"ruff-skill\"]\n\n\
             [[files]]\npath = \"pyproject.toml\"\ntemplate = \"files/pyproject.toml\"\n\n\
             [[sections]]\npath = \"README.md\"\nmarker = \"# Python\"\ntemplate = \"sections/py.md\"\n",
        ),
    ])
    .expect("loads");

    let (skills, files, sections) = resolved_names(&loaded, "base-python");

    assert_eq!(
        skills,
        [
            "tdd=mattpocock/skills#v1.2.3 from base",
            "grilling=mattpocock/skills#v1.2.3 from base",
            "handoff=collectiveai-team/botica from base",
            "ruff-skill=a/b from base-python",
        ]
    );
    assert_eq!(
        files,
        [
            "prek.toml=files/prek-base.toml from base",
            "pyproject.toml=files/pyproject.toml from base-python",
        ]
    );
    assert_eq!(
        sections,
        [
            "AGENTS.md### Botica rules=sections/botica-rules.md from base",
            "README.md## Python=sections/py.md from base-python",
        ]
    );
}

#[test]
fn a_step_that_two_recipes_list_the_same_stays_where_it_first_is() {
    let loaded = project(vec![
        recipe("base", STEPS),
        recipe(
            "again",
            "extends = [\"base\"]\n\n\
             [[skills]]\nsource = \"mattpocock/skills#v1.2.3\"\nnames = [\"tdd\"]\n\n\
             [[files]]\npath = \"prek.toml\"\ntemplate = \"files/prek-base.toml\"\n\n\
             [[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Botica rules\"\n\
             template = \"sections/botica-rules.md\"\n",
        ),
    ])
    .expect("loads");

    let (skills, files, sections) = resolved_names(&loaded, "again");

    assert_eq!(skills.len(), 3);
    assert!(
        skills.iter().all(|s| s.ends_with("from base")),
        "{skills:?}"
    );
    assert_eq!(files, ["prek.toml=files/prek-base.toml from base"]);
    assert_eq!(sections.len(), 1);
}

#[test]
fn a_recipe_that_extends_takes_over_a_skill_from_another_source_and_warns() {
    let loaded = project(vec![
        recipe("base", STEPS),
        recipe(
            "other",
            "extends = [\"base\"]\n\n[[skills]]\nsource = \"fork/skills\"\nnames = [\"handoff\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = loaded.resolve("other").expect("resolves");

    let (skills, _, _) = resolved_names(&loaded, "other");
    assert_eq!(
        skills,
        [
            "tdd=mattpocock/skills#v1.2.3 from base",
            "grilling=mattpocock/skills#v1.2.3 from base",
            "handoff=fork/skills from other",
        ]
    );
    assert_eq!(
        resolved.warnings,
        ["the skill `handoff` comes from `fork/skills` (other), which replaces `collectiveai-team/botica` (base)"]
    );
    assert!(loaded
        .resolve("base")
        .expect("resolves")
        .warnings
        .is_empty());
}

#[test]
fn a_skill_taken_over_twice_keeps_the_last_source_and_warns_each_time() {
    let loaded = project(vec![
        recipe(
            "base",
            "[[skills]]\nsource = \"a/one\"\nnames = [\"x\", \"y\"]\n",
        ),
        recipe(
            "mid",
            "extends = [\"base\"]\n\n[[skills]]\nsource = \"b/two\"\nnames = [\"x\"]\n",
        ),
        recipe(
            "top",
            "extends = [\"mid\"]\n\n[[skills]]\nsource = \"c/three\"\nnames = [\"x\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = loaded.resolve("top").expect("resolves");

    let (skills, _, _) = resolved_names(&loaded, "top");
    assert_eq!(skills, ["x=c/three from top", "y=a/one from base"]);
    assert_eq!(
        resolved.warnings,
        [
            "the skill `x` comes from `b/two` (mid), which replaces `a/one` (base)",
            "the skill `x` comes from `c/three` (top), which replaces `b/two` (mid)",
        ]
    );
}

#[test]
fn a_skill_from_the_same_source_in_two_recipes_does_not_warn() {
    let loaded = project(vec![
        recipe("base", STEPS),
        recipe(
            "again",
            "extends = [\"base\"]\n\n\
             [[skills]]\nsource = \"collectiveai-team/botica\"\nnames = [\"handoff\"]\n",
        ),
    ])
    .expect("loads");

    assert!(loaded
        .resolve("again")
        .expect("resolves")
        .warnings
        .is_empty());
}

#[test]
fn two_recipes_may_add_different_templates_to_the_same_path() {
    let loaded = project(vec![
        recipe("one", "[[files]]\npath = \"a\"\ntemplate = \"files/t\"\n"),
        recipe(
            "two",
            "extends = [\"one\"]\n\n[[files]]\npath = \"a\"\ntemplate = \"files/u\"\n",
        ),
    ])
    .expect("loads");

    let (_, files, _) = resolved_names(&loaded, "two");

    assert_eq!(files, ["a=files/t from one", "a=files/u from two"]);
}

#[test]
fn a_file_is_the_same_only_when_its_path_and_its_template_are() {
    let loaded = project(vec![
        recipe("one", "[[files]]\npath = \"a\"\ntemplate = \"files/t\"\n"),
        recipe(
            "two",
            "extends = [\"one\"]\n\n[[files]]\npath = \"b\"\ntemplate = \"files/t\"\n",
        ),
    ])
    .expect("loads");

    let (_, files, _) = resolved_names(&loaded, "two");

    assert_eq!(files, ["a=files/t from one", "b=files/t from two"]);
}

#[test]
fn a_section_is_the_same_only_when_its_path_its_marker_and_its_template_are() {
    let loaded = project(vec![
        recipe(
            "one",
            "[[sections]]\npath = \"p\"\nmarker = \"m\"\ntemplate = \"sections/t\"\n",
        ),
        recipe(
            "two",
            "extends = [\"one\"]\n\n\
             [[sections]]\npath = \"p\"\nmarker = \"n\"\ntemplate = \"sections/t\"\n\n\
             [[sections]]\npath = \"q\"\nmarker = \"m\"\ntemplate = \"sections/t\"\n",
        ),
        recipe(
            "three",
            "extends = [\"one\"]\n\n\
             [[sections]]\npath = \"p\"\nmarker = \"m\"\ntemplate = \"sections/u\"\n",
        ),
    ])
    .expect("loads");

    let (_, _, two) = resolved_names(&loaded, "two");
    let (_, _, three) = resolved_names(&loaded, "three");

    assert_eq!(
        two,
        [
            "p#m=sections/t from one",
            "p#n=sections/t from two",
            "q#m=sections/t from two"
        ]
    );
    assert_eq!(
        three,
        ["p#m=sections/t from one", "p#m=sections/u from three"]
    );
}
