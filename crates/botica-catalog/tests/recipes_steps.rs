//! The steps of a recipe: skills to install, files and sections to add (ADR 0006).

mod common;

use common::steps::{project, recipe, STEPS};

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}
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
    assert_eq!(base.sections[0].marker, "## Botica rules");
    assert_eq!(base.sections[0].template, "sections/botica-rules.md");
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
        "[[files]]\npath = \"a\"\ntemplate = \"files/t\"\nmode = \"755\"\n",
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"sections/t\"\nafter = \"n\"\n",
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
        "[[files]]\ntemplate = \"files/t\"\n",
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\n",
        "[[sections]]\npath = \"a\"\ntemplate = \"sections/t\"\n",
        "[[sections]]\nmarker = \"m\"\ntemplate = \"sections/t\"\n",
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
            let toml = format!("[[{step}]]\npath = {path:?}\n{marker}template = \"sections/t\"\n");

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
    let toml = "[[files]]\npath = \".github/workflows/ci.yml\"\ntemplate = \"files/t\"\n";

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

    assert!(
        !section_template.contains("names the template"),
        "{section_template}"
    );

    let marker = rejected("[[sections]]\npath = \"a\"\nmarker = \"\"\ntemplate = \"sections/t\"\n");
    assert!(
        marker.contains("the `sections` step for `a` has an empty `marker`"),
        "{marker}"
    );
}

#[test]
fn a_recipe_adds_a_file_once_and_a_section_once() {
    let files = rejected(
        "[[files]]\npath = \"a\"\ntemplate = \"files/t\"\n\n[[files]]\npath = \"a\"\ntemplate = \"files/u\"\n",
    );
    assert!(files.contains("`files` lists `a` twice"), "{files}");

    let sections = rejected(
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"sections/t\"\n\n\
         [[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"sections/u\"\n",
    );
    assert!(
        sections.contains("`sections` lists `m` of `a` twice"),
        "{sections}"
    );

    let other_marker = "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"sections/t\"\n\n\
                        [[sections]]\npath = \"a\"\nmarker = \"n\"\ntemplate = \"sections/t\"\n";
    assert!(project(vec![recipe("x", other_marker)]).is_ok());
}

#[test]
fn every_problem_of_a_recipe_is_reported() {
    let message = rejected(
        "exports = [\"pdf\"]\n\n[[skills]]\nsource = \"\"\nnames = [\"x\"]\n\n\
         [[files]]\npath = \"/a\"\ntemplate = \"files/t\"\n",
    );

    assert!(message.contains("unknown export `pdf`"), "{message}");
    assert!(message.contains("empty `source`"), "{message}");
    assert!(message.contains("`files` path `/a`"), "{message}");
}
