//! `botica recipe show`, `check` and `install` for the `links` step: a symbolic link that is added
//! when its path does not exist and never replaced.
#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::write;

const LINKS: &str = "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n\n\
                     [[links]]\npath = \".claude/skills\"\ntarget = \"../.agents/skills\"\n";

fn setup(name: &str, toml: &str) -> PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "mine", toml);
    dir
}

fn target(dir: &Path, path: &str) -> String {
    fs::read_link(dir.join(path))
        .expect("a link")
        .display()
        .to_string()
}

#[test]
fn show_and_list_say_which_links_a_recipe_adds() {
    let dir = setup("link-show", LINKS);
    add_recipe(
        &dir,
        "child",
        "extends = [\"mine\"]\n[[links]]\npath = \"GEMINI.md\"\ntarget = \"AGENTS.md\"\n",
    );

    let shown = ok(&recipe(&dir, &["show", "child"]));
    let listed = ok(&recipe(&dir, &["list"]));

    assert!(
        shown.ends_with(
            "\n## Links\n\n- CLAUDE.md -> AGENTS.md (from mine)\n\
             - .claude/skills -> ../.agents/skills (from mine)\n\
             - GEMINI.md -> AGENTS.md (from child)\n"
        ),
        "{shown}"
    );
    assert!(listed.contains("  links: 2\n"), "{listed}");
    assert!(listed.contains("  links: 1\n"), "{listed}");
}

#[test]
fn a_link_that_does_not_exist_is_created_with_its_folders() {
    let dir = setup("link-add", LINKS);

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "link CLAUDE.md: added\nlink .claude/skills: added\nmine: 2 added, 0 ok, 0 pending\n"
    );
    assert_eq!(target(&dir, "CLAUDE.md"), "AGENTS.md");
    assert_eq!(target(&dir, ".claude/skills"), "../.agents/skills");
}

#[test]
fn a_link_to_a_target_that_does_not_exist_is_created() {
    let dir = setup("link-dangling", LINKS);

    ok(&recipe(&dir, &["install", "mine"]));

    assert!(!dir.join("AGENTS.md").exists());
    assert!(fs::symlink_metadata(dir.join("CLAUDE.md")).is_ok());
}

#[test]
fn a_link_that_already_points_to_the_target_is_ok() {
    let dir = setup("link-same", LINKS);
    symlink("AGENTS.md", dir.join("CLAUDE.md")).expect("link");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("link CLAUDE.md: ok\n"), "{text}");
    assert!(text.ends_with("mine: 1 added, 1 ok, 0 pending\n"), "{text}");
}

#[test]
fn a_path_that_holds_something_else_is_pending_and_stays_as_it_is() {
    let dir = setup("link-other", LINKS);
    symlink("README.md", dir.join("CLAUDE.md")).expect("link");
    write(&dir, ".claude/skills/inside", "x");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains(
            "link CLAUDE.md: pending: exists and is a link to `README.md`, not to `AGENTS.md`\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("link .claude/skills: pending: exists and is not a link\n"),
        "{text}"
    );
    assert!(text.ends_with("mine: 0 added, 0 ok, 2 pending\n"), "{text}");
    assert_eq!(target(&dir, "CLAUDE.md"), "README.md");
    assert!(dir.join(".claude/skills/inside").is_file());
}

#[test]
fn a_file_at_the_path_is_pending_and_keeps_its_text() {
    let dir = setup(
        "link-file",
        "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n",
    );
    write(&dir, "CLAUDE.md", "my notes\n");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("link CLAUDE.md: pending: exists and is not a link\n"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("CLAUDE.md")).expect("read"),
        "my notes\n"
    );
}

#[test]
fn a_folder_that_cannot_hold_the_link_stops_the_command_with_2() {
    let dir = setup(
        "link-blocked",
        "[[links]]\npath = \".claude/skills\"\ntarget = \"x\"\n",
    );
    write(&dir, ".claude", "a file where a folder should be");

    let text = failed(&recipe(&dir, &["install", "mine"]), 2);

    assert!(text.contains("cannot create"), "{text}");
}

#[test]
fn installing_twice_changes_nothing_the_second_time() {
    let dir = setup("link-twice", LINKS);
    ok(&recipe(&dir, &["install", "mine"]));

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.ends_with("mine: 0 added, 2 ok, 0 pending\n"), "{text}");
}

#[test]
fn the_links_come_after_the_sections_and_before_the_lines_of_gitignore() {
    let dir = setup(
        "link-order",
        &format!(
            "{LINKS}[[gitignore]]\nlines = [\".env\"]\n\
             [[sections]]\npath = \"AGENTS.md\"\nmarker = \"## S\"\ntemplate = \"sections/s.md\"\n"
        ),
    );
    write(&dir, "recipes/sections/s.md", "## S\n");
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["install", "mine"]));

    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {text}"))
    };
    assert!(at("section AGENTS.md") < at("link CLAUDE.md"), "{text}");
    assert!(at("link .claude/skills") < at("gitignore .env"), "{text}");
    assert!(fs::symlink_metadata(dir.join("CLAUDE.md")).is_ok());
    assert!(fs::symlink_metadata(dir.join("deep/er/CLAUDE.md")).is_err());
}

#[test]
fn a_link_that_two_named_recipes_share_is_added_once() {
    let dir = setup(
        "link-shared",
        "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n",
    );
    add_recipe(&dir, "other", LINKS);

    let text = ok(&recipe(&dir, &["install", "mine", "other"]));

    assert_eq!(
        text,
        "link CLAUDE.md: added\nlink .claude/skills: added\nmine, other: 2 added, 0 ok, 0 pending\n"
    );
}

#[test]
fn a_second_link_with_the_same_path_and_another_target_is_pending() {
    let dir = setup(
        "link-same-path",
        "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n",
    );
    add_recipe(
        &dir,
        "child",
        "extends = [\"mine\"]\n[[links]]\npath = \"CLAUDE.md\"\ntarget = \"OTHER.md\"\n",
    );

    let text = failed(&recipe(&dir, &["install", "child"]), 1);

    assert!(
        text.starts_with(
            "link CLAUDE.md: added\nlink CLAUDE.md: pending: exists and is a link to `AGENTS.md`, not to `OTHER.md`\n"
        ),
        "{text}"
    );
}

#[test]
fn check_says_which_links_are_missing_or_point_elsewhere() {
    let dir = setup("link-check", LINKS);
    symlink("README.md", dir.join("CLAUDE.md")).expect("link");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert_eq!(
        text,
        "link CLAUDE.md: differs: is a link to `README.md`, not to `AGENTS.md`\n\
         link .claude/skills: differs: does not exist\n\
         mine: 2 checked, 2 differ\n"
    );
}

#[test]
fn check_says_when_the_path_is_not_a_link_and_is_ok_after_install() {
    let dir = setup("link-check-file", LINKS);
    write(&dir, "CLAUDE.md", "x");
    let text = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        text.contains("link CLAUDE.md: differs: is not a link\n"),
        "{text}"
    );

    let dir = setup("link-check-ok", LINKS);
    ok(&recipe(&dir, &["install", "mine"]));
    let text = ok(&recipe(&dir, &["check", "mine"]));
    assert_eq!(
        text,
        "link CLAUDE.md: ok\nlink .claude/skills: ok\nmine: 2 checked, 0 differ\n"
    );
}

#[test]
fn check_goes_through_the_links_from_the_root_of_the_project() {
    let dir = setup(
        "link-check-root",
        "[[links]]\npath = \"CLAUDE.md\"\ntarget = \"AGENTS.md\"\n",
    );
    symlink("AGENTS.md", dir.join("CLAUDE.md")).expect("link");
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["check", "mine"]));

    assert!(text.ends_with("mine: 1 checked, 0 differ\n"), "{text}");
}
