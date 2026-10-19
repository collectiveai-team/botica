//! The `gitignore` step of a recipe: lines to add to `.gitignore` and lines to stop ignoring
//! (ADR 0006).

mod common;

use common::steps::{project, recipe};

const STEP: &str = "[[gitignore]]\nlines = [\".env\", \"!.env.schema\", \".tmp/\"]\n\
                    unignore = [\"skills-lock.json\"]\n";

fn rejected(toml: &str) -> String {
    project(vec![recipe("x", toml)])
        .expect_err("rejected")
        .to_string()
}

#[test]
fn loads_the_lines_and_the_unignored_lines_of_a_step() {
    let loaded = project(vec![recipe("base", STEP)]).expect("loads");

    let base = loaded.recipe("base").expect("the recipe");
    assert_eq!(base.gitignore.len(), 1);
    assert_eq!(base.gitignore[0].lines, [".env", "!.env.schema", ".tmp/"]);
    assert_eq!(base.gitignore[0].unignore, ["skills-lock.json"]);
}

#[test]
fn each_list_of_a_step_is_optional_but_one_is_needed() {
    let loaded = project(vec![recipe(
        "base",
        "[[gitignore]]\nlines = [\"a\"]\n\n[[gitignore]]\nunignore = [\"skills-lock.json\"]\n",
    )])
    .expect("loads");
    let base = loaded.recipe("base").expect("the recipe");
    assert!(base.gitignore[0].unignore.is_empty());
    assert!(base.gitignore[1].lines.is_empty());

    let message = rejected("[[gitignore]]\n");
    assert!(
        message.contains("a `gitignore` step has no `lines` and no `unignore`"),
        "{message}"
    );
}

#[test]
fn a_recipe_without_the_step_has_none() {
    let loaded = project(vec![recipe("bare", "rules = [\"a\"]\n")]).expect("loads");

    assert!(loaded
        .recipe("bare")
        .expect("the recipe")
        .gitignore
        .is_empty());
}

#[test]
fn an_unknown_key_of_the_step_is_rejected() {
    let message = rejected("[[gitignore]]\nlines = [\"a\"]\nafter = \"b\"\n");

    assert!(message.contains("x.toml"), "{message}");
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn a_line_is_not_empty_and_is_one_line() {
    let empty = rejected("[[gitignore]]\nlines = [\" \"]\n");
    assert!(empty.contains("`gitignore` has an empty line"), "{empty}");

    for line in ["a\\nb", "a\\rb"] {
        let two = rejected(&format!("[[gitignore]]\nlines = [\"{line}\"]\n"));
        assert!(two.contains("a `gitignore` line has a line break"), "{two}");
    }
}

#[test]
fn a_line_is_listed_once_in_a_recipe_even_across_steps() {
    let message = rejected("[[gitignore]]\nlines = [\"a\"]\n\n[[gitignore]]\nlines = [\"a\"]\n");

    assert!(message.contains("`gitignore` lists `a` twice"), "{message}");
}

#[test]
fn only_the_lines_of_the_whitelist_can_be_unignored() {
    let message = rejected("[[gitignore]]\nunignore = [\".env\"]\n");

    assert!(
        message.contains("`unignore` can only name `skills-lock.json`, not `.env`"),
        "{message}"
    );
}

#[test]
fn an_unignored_line_is_listed_once() {
    let message = rejected(
        "[[gitignore]]\nunignore = [\"skills-lock.json\"]\n\n\
         [[gitignore]]\nunignore = [\"skills-lock.json\"]\n",
    );

    assert!(
        message.contains("`unignore` lists `skills-lock.json` twice"),
        "{message}"
    );
}

#[test]
fn the_manifest_of_skills_is_never_a_line_to_ignore() {
    for line in ["skills-lock.json", " skills-lock.json "] {
        let message = rejected(&format!("[[gitignore]]\nlines = [\"{line}\"]\n"));

        assert!(
            message.contains("`gitignore` cannot ignore `skills-lock.json`"),
            "{message}"
        );
    }
}

#[test]
fn a_recipe_resolves_the_lines_of_what_it_extends_first_and_each_line_once() {
    let loaded = project(vec![
        recipe("base", STEP),
        recipe(
            "child",
            "extends = [\"base\"]\n\n[[gitignore]]\nlines = [\".tmp/\", \".venv/\"]\n\
             unignore = [\"skills-lock.json\"]\n",
        ),
    ])
    .expect("loads");

    let resolved = loaded.resolve("child").expect("resolves");

    let lines: Vec<String> = resolved
        .gitignore
        .iter()
        .map(|item| format!("{} from {}", item.id, item.from))
        .collect();
    assert_eq!(
        lines,
        [
            ".env from base",
            "!.env.schema from base",
            ".tmp/ from base",
            ".venv/ from child"
        ]
    );
    let unignored: Vec<String> = resolved
        .unignore
        .iter()
        .map(|item| format!("{} from {}", item.id, item.from))
        .collect();
    assert_eq!(unignored, ["skills-lock.json from base"]);
}
