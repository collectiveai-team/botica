//! `botica recipe list` and `botica recipe show` print the steps of a recipe.

mod common;

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::stderr;

const BASE: &str = r###"description = "Any repository"
rules = ["file-size-guard"]

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
fn list_counts_the_skills_the_files_and_the_sections_of_a_recipe_that_has_steps() {
    let dir = project("steps-list", None);
    add_recipe(&dir, "mine", BASE);
    add_recipe(&dir, "plain", "rules = [\"file-size-guard\"]\n");

    let text = ok(&recipe(&dir, &["list"]));

    assert!(
        text.contains(
            "mine: Any repository\n  rules: 1, judgment: 0, exports: 0\n  \
             skills: 3, files: 1, sections: 1\n  from: project\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("plain\n  rules: 1, judgment: 0, exports: 0\n  from: project\n"),
        "{text}"
    );
}

#[test]
fn show_prints_each_step_with_the_recipe_that_lists_it() {
    let dir = project("steps-show", None);
    add_recipe(&dir, "mine", BASE);
    add_recipe(
        &dir,
        "child",
        "extends = [\"mine\"]\n\n[[skills]]\nsource = \"a/b\"\nnames = [\"extra\"]\n\n\
         [[files]]\npath = \"pyproject.toml\"\ntemplate = \"files/pyproject.toml\"\n",
    );

    let text = ok(&recipe(&dir, &["show", "child"]));

    assert!(
        text.contains(
            "\n## Skills\n\n- tdd (mattpocock/skills#v1.2.3, from mine)\n\
             - grilling (mattpocock/skills#v1.2.3, from mine)\n\
             - handoff (collectiveai-team/botica, from mine)\n\
             - extra (a/b, from child)\n"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "\n## Files\n\n- prek.toml (template files/prek-base.toml, from mine)\n\
             - pyproject.toml (template files/pyproject.toml, from child)\n"
        ),
        "{text}"
    );
    assert!(
        text.ends_with(
            "\n## Sections\n\n- AGENTS.md, `## Engineering Standards` \
             (template sections/standards-index.md, from mine)\n"
        ),
        "{text}"
    );
    let exports = text.find("## Exports").expect("exports");
    let skills = text.find("## Skills").expect("skills");
    let files = text.find("## Files").expect("files");
    let sections = text.find("## Sections").expect("sections");
    assert!(
        exports < skills && skills < files && files < sections,
        "{text}"
    );
}

#[test]
fn show_leaves_out_the_steps_a_recipe_does_not_have() {
    let dir = project("steps-show-none", None);
    add_recipe(&dir, "plain", "rules = [\"file-size-guard\"]\n");
    add_recipe(
        &dir,
        "only-files",
        "[[files]]\npath = \"a\"\ntemplate = \"t\"\n",
    );

    let plain = ok(&recipe(&dir, &["show", "plain"]));
    let only = ok(&recipe(&dir, &["show", "only-files"]));

    assert!(
        !plain.contains("## Skills") && !plain.contains("## Files"),
        "{plain}"
    );
    assert!(!plain.contains("## Sections"), "{plain}");
    assert!(
        only.contains("## Files") && !only.contains("## Skills"),
        "{only}"
    );
    assert!(!only.contains("## Sections"), "{only}");
}

#[test]
fn show_prints_the_skill_of_the_recipe_that_takes_it_over_and_warns_on_stderr() {
    let dir = project("steps-show-takeover", None);
    add_recipe(&dir, "mine", BASE);
    add_recipe(
        &dir,
        "fork",
        "extends = [\"mine\"]\n\n[[skills]]\nsource = \"fork/skills\"\nnames = [\"tdd\"]\n",
    );

    let output = recipe(&dir, &["show", "fork"]);

    let text = ok(&output);
    assert!(text.contains("- tdd (fork/skills, from fork)\n"), "{text}");
    assert!(!text.contains("warning"), "{text}");
    assert_eq!(
        stderr(&output),
        "warning: the skill `tdd` comes from `fork/skills` (fork), which replaces \
         `mattpocock/skills#v1.2.3` (mine)\n"
    );
    assert_eq!(stderr(&recipe(&dir, &["show", "mine"])), "");
}

#[test]
fn a_step_with_a_problem_stops_the_recipes_from_loading() {
    let dir = project("steps-bad", None);
    add_recipe(
        &dir,
        "broken",
        "[[files]]\npath = \"/etc/x\"\ntemplate = \"t\"\n",
    );

    let text = failed(&recipe(&dir, &["list"]), 2);

    assert!(text.contains("broken.toml"), "{text}");
    assert!(text.contains("a `files` path `/etc/x`"), "{text}");
}

#[test]
fn list_counts_a_recipe_that_has_only_one_kind_of_step() {
    let dir = project("steps-list-one", None);
    add_recipe(
        &dir,
        "only-skill",
        "[[skills]]\nsource = \"a/b\"\nnames = [\"x\"]\n",
    );
    add_recipe(
        &dir,
        "only-file",
        "[[files]]\npath = \"a\"\ntemplate = \"t\"\n",
    );
    add_recipe(
        &dir,
        "only-section",
        "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"t\"\n",
    );

    let text = ok(&recipe(&dir, &["list"]));

    for (id, counts) in [
        ("only-skill", "skills: 1, files: 0, sections: 0"),
        ("only-file", "skills: 0, files: 1, sections: 0"),
        ("only-section", "skills: 0, files: 0, sections: 1"),
    ] {
        let block = format!("{id}\n  rules: 0, judgment: 0, exports: 0\n  {counts}\n");
        assert!(text.contains(&block), "{id}: {text}");
    }
}
