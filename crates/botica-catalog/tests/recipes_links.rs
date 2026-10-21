//! The `links` step of a recipe: a symbolic link to add, such as `CLAUDE.md` to `AGENTS.md`
//! (ADR 0006).

mod common;

use common::steps::{project, recipe};

const STEP: &str = "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n\n\
                    [[links]]\npath = \".claude/skills\"\ntarget = \"../.agents/skills\"\n";

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}

#[test]
fn loads_the_links_of_a_recipe_in_order() {
    let loaded = project(vec![recipe("base", STEP)]).expect("loads");

    let base = loaded.recipe("base").expect("the recipe");
    assert_eq!(base.links.len(), 2);
    assert_eq!(base.links[0].path, "CLAUDE.md");
    assert_eq!(base.links[0].target, "AGENTS.md");
    assert_eq!(base.links[1].path, ".claude/skills");
    assert_eq!(base.links[1].target, "../.agents/skills");
}

#[test]
fn a_recipe_without_links_has_none() {
    let loaded = project(vec![recipe("bare", "rules = [\"a\"]\n")]).expect("loads");

    assert!(loaded.recipe("bare").expect("the recipe").links.is_empty());
}

#[test]
fn an_unknown_key_is_rejected_and_each_field_is_needed() {
    let unknown = rejected("[[links]]\npath = \"a\"\ntarget = \"b\"\nforce = true\n");
    assert!(unknown.contains("unknown field"), "{unknown}");

    for toml in ["[[links]]\npath = \"a\"\n", "[[links]]\ntarget = \"b\"\n"] {
        let message = rejected(toml);
        assert!(message.contains("x.toml"), "{message}");
        assert!(message.contains("missing field"), "{message}");
    }
}

#[test]
fn the_path_is_relative_to_the_root_of_the_project() {
    for path in ["/a", "../a", "a/../b", "a//b", "./a", "a\\\\b", " "] {
        let message = rejected(&format!("[[links]]\npath = \"{path}\"\ntarget = \"b\"\n"));

        assert!(message.contains("a `links` path"), "{path}: {message}");
    }
}

#[test]
fn the_target_is_a_relative_path_that_may_go_up() {
    let loaded = project(vec![recipe(
        "x",
        "[[links]]\npath = \"a/b\"\ntarget = \"../c/d\"\n",
    )])
    .expect("loads");
    assert_eq!(
        loaded.recipe("x").expect("recipe").links[0].target,
        "../c/d"
    );

    for target in ["", " ", "/etc/passwd", "a\\\\b", "a//b"] {
        let message = rejected(&format!("[[links]]\npath = \"a\"\ntarget = \"{target}\"\n"));

        assert!(
            message.contains("the `links` step for `a` has a target"),
            "{target:?}: {message}"
        );
    }
}

#[test]
fn a_path_is_listed_once_in_a_recipe() {
    let message = rejected(
        "[[links]]\npath = \"a\"\ntarget = \"b\"\n\n[[links]]\npath = \"a\"\ntarget = \"c\"\n",
    );

    assert!(message.contains("`links` lists `a` twice"), "{message}");
}

#[test]
fn a_recipe_resolves_the_links_of_what_it_extends_first_and_a_shared_link_once() {
    let loaded = project(vec![
        recipe("base", STEP),
        recipe(
            "child",
            "extends = [\"base\"]\n\n[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n\n\
             [[links]]\npath = \"GEMINI.md\"\ntarget = \"AGENTS.md\"\n\n\
             [[links]]\npath = \".claude/skills\"\ntarget = \"other\"\n",
        ),
    ])
    .expect("loads");

    let resolved = loaded.resolve("child").expect("resolves");

    let links: Vec<String> = resolved
        .links
        .iter()
        .map(|link| format!("{}->{} from {}", link.path, link.target, link.from))
        .collect();
    assert_eq!(
        links,
        [
            "CLAUDE.md->AGENTS.md from base",
            ".claude/skills->../.agents/skills from base",
            "GEMINI.md->AGENTS.md from child",
            ".claude/skills->other from child",
        ]
    );
}
