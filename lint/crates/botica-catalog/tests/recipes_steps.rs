//! The steps of a recipe: skills to install, files and sections to add (ADR 0006).

mod common;

use botica_catalog::{CatalogError, Recipes, SourceFile};
use common::file;

fn recipe(id: &str, toml: &str) -> SourceFile {
    file(&format!("{id}.toml"), toml)
}

fn project(files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    Recipes::from_files(&files)
}

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}

const STEPS: &str = r###"
[[skills]]
source = "mattpocock/skills#v1.2.3"
names = ["tdd", "grilling"]

[[skills]]
source = "collectiveai-team/botica"
names = ["handoff"]

[[files]]
path = "prek.toml"
template = "files/prek-base.toml"

[[sections]]
path = "AGENTS.md"
marker = "## Engineering Standards"
template = "sections/standards-index.md"
"###;

#[test]
fn loads_the_skills_the_files_and_the_sections_of_a_recipe_in_order() {
    let loaded = project(vec![recipe("base", STEPS)]).expect("loads");

    let base = loaded.recipe("base").expect("the recipe");
    assert_eq!(base.skills.len(), 2);
    assert_eq!(base.skills[0].source, "mattpocock/skills#v1.2.3");
    assert_eq!(base.skills[0].names, ["tdd", "grilling"]);
    assert_eq!(base.skills[1].source, "collectiveai-team/botica");
    assert_eq!(base.skills[1].names, ["handoff"]);
    assert_eq!(base.files.len(), 1);
    assert_eq!(base.files[0].path, "prek.toml");
    assert_eq!(base.files[0].template, "files/prek-base.toml");
    assert_eq!(base.sections.len(), 1);
    assert_eq!(base.sections[0].path, "AGENTS.md");
    assert_eq!(base.sections[0].marker, "## Engineering Standards");
    assert_eq!(base.sections[0].template, "sections/standards-index.md");
}

#[test]
fn a_recipe_without_steps_has_none() {
    let loaded = project(vec![recipe("bare", "rules = [\"a\"]\n")]).expect("loads");

    let bare = loaded.recipe("bare").expect("the recipe");
    assert!(bare.skills.is_empty() && bare.files.is_empty() && bare.sections.is_empty());
}

#[test]
fn an_unknown_key_of_a_step_is_rejected() {
    for toml in [
        "[[skills]]\nsource = \"a/b\"\nnames = [\"x\"]\nref = \"v1\"\n",
        "[[files]]\npath = \"a\"\ntemplate = \"t\"\nmode = \"755\"\n",
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"t\"\nafter = \"n\"\n",
    ] {
        let message = rejected(toml);

        assert!(message.contains("x.toml"), "{message}");
    }
}

#[test]
fn a_step_needs_each_of_its_fields() {
    for toml in [
        "[[skills]]\nnames = [\"x\"]\n",
        "[[skills]]\nsource = \"a/b\"\n",
        "[[files]]\npath = \"a\"\n",
        "[[files]]\ntemplate = \"t\"\n",
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\n",
        "[[sections]]\npath = \"a\"\ntemplate = \"t\"\n",
        "[[sections]]\nmarker = \"m\"\ntemplate = \"t\"\n",
    ] {
        let message = rejected(toml);

        assert!(message.contains("x.toml"), "{toml}: {message}");
        assert!(message.contains("missing field"), "{toml}: {message}");
    }
}

#[test]
fn a_skill_step_has_a_source_and_names_that_are_not_empty() {
    let source = rejected("[[skills]]\nsource = \" \"\nnames = [\"x\"]\n");
    assert!(
        source.contains("a `skills` step has an empty `source`"),
        "{source}"
    );

    let none = rejected("[[skills]]\nsource = \"a/b\"\nnames = []\n");
    assert!(
        none.contains("the `skills` step for `a/b` has no names"),
        "{none}"
    );

    let blank = rejected("[[skills]]\nsource = \"a/b\"\nnames = [\"x\", \"\"]\n");
    assert!(
        blank.contains("the `skills` step for `a/b` has an empty name"),
        "{blank}"
    );

    let spaced = rejected("[[skills]]\nsource = \"a/b\"\nnames = [\"two words\"]\n");
    assert!(
        spaced.contains("the skill name `two words` has a space"),
        "{spaced}"
    );
}

#[test]
fn a_recipe_names_a_skill_once_even_across_steps() {
    let same_step = rejected("[[skills]]\nsource = \"a/b\"\nnames = [\"x\", \"x\"]\n");
    assert!(
        same_step.contains("`skills` lists `x` twice"),
        "{same_step}"
    );

    let two_steps = rejected(
        "[[skills]]\nsource = \"a/b\"\nnames = [\"x\"]\n\n\
         [[skills]]\nsource = \"c/d\"\nnames = [\"x\"]\n",
    );
    assert!(
        two_steps.contains("`skills` lists `x` twice"),
        "{two_steps}"
    );
}

#[test]
fn a_path_is_relative_to_the_root_of_the_project() {
    for path in [
        "",
        " ",
        "/etc/passwd",
        "../up",
        "a/../b",
        "a//b",
        "./a",
        "a\\b",
        "a/",
    ] {
        for step in ["files", "sections"] {
            let marker = if step == "sections" {
                "marker = \"m\"\n"
            } else {
                ""
            };
            let toml = format!("[[{step}]]\npath = {path:?}\n{marker}template = \"t\"\n");

            let message = rejected(&toml);

            assert!(
                message.contains(&format!("a `{step}` path `{path}` must be a path")),
                "{path:?} in {step}: {message}"
            );
        }
    }
}

#[test]
fn a_nested_path_is_accepted() {
    let toml = "[[files]]\npath = \".github/workflows/ci.yml\"\ntemplate = \"t\"\n";

    assert!(project(vec![recipe("x", toml)]).is_ok());
}

#[test]
fn a_template_and_a_marker_are_not_empty() {
    let file_template = rejected("[[files]]\npath = \"a\"\ntemplate = \"\"\n");
    assert!(
        file_template.contains("the `files` step for `a` has an empty `template`"),
        "{file_template}"
    );

    let section_template =
        rejected("[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \" \"\n");
    assert!(
        section_template.contains("the `sections` step for `a` has an empty `template`"),
        "{section_template}"
    );

    let marker = rejected("[[sections]]\npath = \"a\"\nmarker = \"\"\ntemplate = \"t\"\n");
    assert!(
        marker.contains("the `sections` step for `a` has an empty `marker`"),
        "{marker}"
    );
}

#[test]
fn a_recipe_adds_a_file_once_and_a_section_once() {
    let files = rejected(
        "[[files]]\npath = \"a\"\ntemplate = \"t\"\n\n[[files]]\npath = \"a\"\ntemplate = \"u\"\n",
    );
    assert!(files.contains("`files` lists `a` twice"), "{files}");

    let sections = rejected(
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"t\"\n\n\
         [[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"u\"\n",
    );
    assert!(
        sections.contains("`sections` lists `m` of `a` twice"),
        "{sections}"
    );

    let other_marker = "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"t\"\n\n\
                        [[sections]]\npath = \"a\"\nmarker = \"n\"\ntemplate = \"t\"\n";
    assert!(project(vec![recipe("x", other_marker)]).is_ok());
}

#[test]
fn every_problem_of_a_recipe_is_reported() {
    let message = rejected(
        "exports = [\"pdf\"]\n\n[[skills]]\nsource = \"\"\nnames = [\"x\"]\n\n\
         [[files]]\npath = \"/a\"\ntemplate = \"t\"\n",
    );

    assert!(message.contains("unknown export `pdf`"), "{message}");
    assert!(message.contains("empty `source`"), "{message}");
    assert!(message.contains("`files` path `/a`"), "{message}");
}

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
            "AGENTS.md### Engineering Standards=sections/standards-index.md from base",
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
             [[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Engineering Standards\"\n\
             template = \"sections/standards-index.md\"\n",
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
        recipe("one", "[[files]]\npath = \"a\"\ntemplate = \"t\"\n"),
        recipe(
            "two",
            "extends = [\"one\"]\n\n[[files]]\npath = \"a\"\ntemplate = \"u\"\n",
        ),
    ])
    .expect("loads");

    let (_, files, _) = resolved_names(&loaded, "two");

    assert_eq!(files, ["a=t from one", "a=u from two"]);
}

#[test]
fn a_file_is_the_same_only_when_its_path_and_its_template_are() {
    let loaded = project(vec![
        recipe("one", "[[files]]\npath = \"a\"\ntemplate = \"t\"\n"),
        recipe(
            "two",
            "extends = [\"one\"]\n\n[[files]]\npath = \"b\"\ntemplate = \"t\"\n",
        ),
    ])
    .expect("loads");

    let (_, files, _) = resolved_names(&loaded, "two");

    assert_eq!(files, ["a=t from one", "b=t from two"]);
}

#[test]
fn a_section_is_the_same_only_when_its_path_its_marker_and_its_template_are() {
    let loaded = project(vec![
        recipe(
            "one",
            "[[sections]]\npath = \"p\"\nmarker = \"m\"\ntemplate = \"t\"\n",
        ),
        recipe(
            "two",
            "extends = [\"one\"]\n\n\
             [[sections]]\npath = \"p\"\nmarker = \"n\"\ntemplate = \"t\"\n\n\
             [[sections]]\npath = \"q\"\nmarker = \"m\"\ntemplate = \"t\"\n",
        ),
        recipe(
            "three",
            "extends = [\"one\"]\n\n\
             [[sections]]\npath = \"p\"\nmarker = \"m\"\ntemplate = \"u\"\n",
        ),
    ])
    .expect("loads");

    let (_, _, two) = resolved_names(&loaded, "two");
    let (_, _, three) = resolved_names(&loaded, "three");

    assert_eq!(two, ["p#m=t from one", "p#n=t from two", "q#m=t from two"]);
    assert_eq!(three, ["p#m=t from one", "p#m=u from three"]);
}
