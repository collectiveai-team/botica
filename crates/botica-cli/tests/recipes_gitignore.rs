//! `botica recipe install` and `check` for the `gitignore` step: lines that are added to
//! `.gitignore` when missing, and the manifest of skills that is let out of it (ADR 0001).

mod common;

use std::fs;
use std::path::Path;

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::write;

const STEP: &str = "[[gitignore]]\nlines = [\".env\", \"!.env.schema\", \".tmp/\"]\n";
const UNIGNORE: &str = "[[gitignore]]\nunignore = [\"skills-lock.json\"]\n";

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).expect("read the file")
}

fn setup(name: &str, toml: &str) -> std::path::PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "mine", toml);
    dir
}

#[test]
fn show_and_list_say_which_lines_a_recipe_adds_and_unignores() {
    let dir = setup("gi-show", &format!("{STEP}{UNIGNORE}"));
    add_recipe(
        &dir,
        "child",
        "extends = [\"mine\"]\n[[gitignore]]\nlines = [\".venv/\"]\n",
    );

    let shown = ok(&recipe(&dir, &["show", "child"]));
    let listed = ok(&recipe(&dir, &["list"]));

    assert!(
        shown.contains(
            "\n## Gitignore\n\n- .env (from mine)\n- !.env.schema (from mine)\n\
             - .tmp/ (from mine)\n- .venv/ (from child)\n\n## Unignore\n\n\
             - skills-lock.json (from mine)\n"
        ),
        "{shown}"
    );
    assert!(listed.contains("mine\n"), "{listed}");
    assert!(listed.contains("  gitignore: 4\n"), "{listed}");
    assert!(listed.contains("  gitignore: 1\n"), "{listed}");
}

#[test]
fn list_leaves_out_the_kinds_of_step_a_recipe_does_not_have() {
    let dir = setup("gi-list", STEP);

    let listed = ok(&recipe(&dir, &["list"]));

    assert!(listed.contains("  gitignore: 3\n"), "{listed}");
    assert!(!listed.contains("skills: 0"), "{listed}");
    assert!(!listed.contains("files: 0"), "{listed}");
}

#[test]
fn a_gitignore_that_does_not_exist_is_written_with_the_lines() {
    let dir = setup("gi-create", STEP);

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "gitignore .env: added\ngitignore !.env.schema: added\ngitignore .tmp/: added\n\
         mine: 3 added, 0 ok, 0 pending\n"
    );
    assert_eq!(read(&dir, ".gitignore"), ".env\n!.env.schema\n.tmp/\n");
}

#[test]
fn only_the_missing_lines_are_appended_to_a_gitignore_that_exists() {
    let dir = setup("gi-append", STEP);
    write(&dir, ".gitignore", "target/\n.env\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "gitignore .env: ok\ngitignore !.env.schema: added\ngitignore .tmp/: added\n\
         mine: 2 added, 1 ok, 0 pending\n"
    );
    assert_eq!(
        read(&dir, ".gitignore"),
        "target/\n.env\n!.env.schema\n.tmp/\n"
    );
}

#[test]
fn a_gitignore_without_a_last_line_break_gets_one_before_the_lines() {
    let dir = setup("gi-no-newline", "[[gitignore]]\nlines = [\".tmp/\"]\n");
    write(&dir, ".gitignore", "target/");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir, ".gitignore"), "target/\n.tmp/\n");
}

#[test]
fn an_empty_gitignore_gets_the_lines_without_a_blank_line() {
    let dir = setup("gi-empty", "[[gitignore]]\nlines = [\".tmp/\"]\n");
    write(&dir, ".gitignore", "");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir, ".gitignore"), ".tmp/\n");
}

#[test]
fn a_line_with_white_space_at_its_end_counts_but_one_with_it_at_its_start_does_not() {
    let dir = setup("gi-white", "[[gitignore]]\nlines = [\".env\", \".tmp/\"]\n");
    write(&dir, ".gitignore", ".env  \n .tmp/\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("gitignore .env: ok\n"), "{text}");
    assert!(text.contains("gitignore .tmp/: added\n"), "{text}");
    assert_eq!(read(&dir, ".gitignore"), ".env  \n .tmp/\n.tmp/\n");
}

#[test]
fn a_gitignore_that_has_every_line_is_not_touched() {
    let dir = setup("gi-all", STEP);
    write(
        &dir,
        ".gitignore",
        "# mine\r\n.env\r\n!.env.schema\r\n.tmp/",
    );

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.ends_with("mine: 0 added, 3 ok, 0 pending\n"), "{text}");
    assert_eq!(
        read(&dir, ".gitignore"),
        "# mine\r\n.env\r\n!.env.schema\r\n.tmp/"
    );
}

#[test]
fn a_gitignore_that_is_not_a_file_or_cannot_be_read_leaves_each_line_pending() {
    let dir = setup("gi-dir", STEP);
    write(&dir, ".gitignore/inside", "x");
    let text = failed(&recipe(&dir, &["install", "mine"]), 1);
    assert!(
        text.contains("gitignore .env: pending: .gitignore is not a file\n"),
        "{text}"
    );
    assert!(text.ends_with("mine: 0 added, 0 ok, 3 pending\n"), "{text}");

    let dir = setup("gi-binary", STEP);
    common::write_bytes(&dir, ".gitignore", &[0xff, 0xfe, 0x00]);
    let text = failed(&recipe(&dir, &["install", "mine"]), 1);
    assert!(
        text.contains("gitignore .env: pending: cannot read .gitignore: "),
        "{text}"
    );
    assert_eq!(
        fs::read(dir.join(".gitignore")).expect("read"),
        [0xff, 0xfe, 0x00]
    );
}

#[test]
fn a_line_is_added_to_a_gitignore_that_does_not_exist_whatever_the_line_is() {
    let dir = setup("gi-x", "[[gitignore]]\nlines = [\"x\"]\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("gitignore x: added\n"), "{text}");
    assert_eq!(read(&dir, ".gitignore"), "x\n");
}

#[cfg(unix)]
#[test]
fn a_gitignore_that_is_a_link_to_nothing_is_pending_and_nothing_is_written_through_it() {
    let dir = setup("gi-link", &format!("{STEP}{UNIGNORE}"));
    std::os::unix::fs::symlink("missing", dir.join(".gitignore")).expect("link");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("gitignore .env: pending: .gitignore is not a file\n"),
        "{text}"
    );
    assert!(!dir.join("missing").exists());
}

#[test]
fn installing_twice_changes_nothing_the_second_time() {
    let dir = setup("gi-twice", STEP);
    ok(&recipe(&dir, &["install", "mine"]));

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.ends_with("mine: 0 added, 3 ok, 0 pending\n"), "{text}");
    assert_eq!(read(&dir, ".gitignore"), ".env\n!.env.schema\n.tmp/\n");
}

#[test]
fn the_lines_come_after_the_sections_and_the_path_is_from_the_root_of_the_project() {
    let dir = setup(
        "gi-root",
        &format!(
            "{STEP}[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## S\"\ntemplate = \"sections/s.md\"\n"
        ),
    );
    write(&dir, "recipes/sections/s.md", "## S\n");
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["install", "mine"]));

    assert!(
        text.find("section AGENTS.md").expect("section")
            < text.find("gitignore .env").expect("line"),
        "{text}"
    );
    assert!(dir.join(".gitignore").is_file());
    assert!(!dir.join("deep/er/.gitignore").exists());
}

#[test]
fn a_line_that_two_named_recipes_share_is_added_once() {
    let dir = setup("gi-shared", "[[gitignore]]\nlines = [\".env\"]\n");
    add_recipe(
        &dir,
        "other",
        "[[gitignore]]\nlines = [\".env\", \".venv/\"]\n",
    );

    let text = ok(&recipe(&dir, &["install", "mine", "other"]));

    assert_eq!(
        text,
        "gitignore .env: added\ngitignore .venv/: added\nmine, other: 2 added, 0 ok, 0 pending\n"
    );
    assert_eq!(read(&dir, ".gitignore"), ".env\n.venv/\n");
}

#[test]
fn the_manifest_line_is_removed_and_the_rest_of_the_file_is_kept_byte_for_byte() {
    let dir = setup("gi-unignore", UNIGNORE);
    write(
        &dir,
        ".gitignore",
        "a\r\n  skills-lock.json  \r\nb\r\nskills-lock.json",
    );

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "unignore skills-lock.json: edited: removed the line from .gitignore\n\
         mine: 0 added, 0 ok, 0 pending, 1 edited\n"
    );
    assert_eq!(read(&dir, ".gitignore"), "a\r\nb\r\n");
}

#[test]
fn a_gitignore_without_the_manifest_line_or_without_a_file_is_ok() {
    let dir = setup("gi-unignore-ok", UNIGNORE);
    let text = ok(&recipe(&dir, &["install", "mine"]));
    assert_eq!(
        text,
        "unignore skills-lock.json: ok\nmine: 0 added, 1 ok, 0 pending\n"
    );
    assert!(!dir.join(".gitignore").exists());

    write(
        &dir,
        ".gitignore",
        "*.json\n/skills-lock.json\nx-skills-lock.json\n",
    );
    let text = ok(&recipe(&dir, &["install", "mine"]));
    assert!(text.contains("unignore skills-lock.json: ok\n"), "{text}");
    assert_eq!(
        read(&dir, ".gitignore"),
        "*.json\n/skills-lock.json\nx-skills-lock.json\n"
    );
}

#[test]
fn a_gitignore_that_cannot_be_read_or_is_not_a_file_leaves_the_unignore_pending() {
    let dir = setup("gi-unignore-binary", UNIGNORE);
    common::write_bytes(&dir, ".gitignore", &[0xff, 0xfe, 0x00]);

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("unignore skills-lock.json: pending: cannot read .gitignore: "),
        "{text}"
    );

    let dir = setup("gi-unignore-dir", UNIGNORE);
    write(&dir, ".gitignore/inside", "x");
    let text = failed(&recipe(&dir, &["install", "mine"]), 1);
    assert!(
        text.contains("unignore skills-lock.json: pending: .gitignore is not a file\n"),
        "{text}"
    );
}

#[test]
fn check_says_which_lines_are_missing_and_whether_the_manifest_is_still_ignored() {
    let dir = setup("gi-check", &format!("{STEP}{UNIGNORE}"));
    write(&dir, ".gitignore", ".env\n  skills-lock.json\n");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert_eq!(
        text,
        "gitignore .env: ok\n\
         gitignore !.env.schema: differs: not in .gitignore\n\
         gitignore .tmp/: differs: not in .gitignore\n\
         unignore skills-lock.json: differs: .gitignore still has the line\n\
         mine: 4 checked, 3 differ\n"
    );
}

#[test]
fn check_is_ok_after_install() {
    let dir = setup("gi-check-ok", &format!("{STEP}{UNIGNORE}"));
    write(&dir, ".gitignore", "skills-lock.json\n");
    ok(&recipe(&dir, &["install", "mine"]));

    let text = ok(&recipe(&dir, &["check", "mine"]));

    assert_eq!(
        text,
        "gitignore .env: ok\ngitignore !.env.schema: ok\ngitignore .tmp/: ok\n\
         unignore skills-lock.json: ok\nmine: 4 checked, 0 differ\n"
    );
}

#[test]
fn check_without_a_gitignore_says_so_and_the_unignore_is_ok() {
    let dir = setup("gi-check-none", &format!("{STEP}{UNIGNORE}"));

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("gitignore .env: differs: no .gitignore\n"),
        "{text}"
    );
    assert!(text.contains("unignore skills-lock.json: ok\n"), "{text}");
}

#[test]
fn check_says_when_the_gitignore_is_not_a_file_or_cannot_be_read() {
    let dir = setup("gi-check-dir", &format!("{STEP}{UNIGNORE}"));
    write(&dir, ".gitignore/inside", "x");
    let text = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        text.contains("gitignore .env: differs: .gitignore is not a file\n"),
        "{text}"
    );
    assert!(
        text.contains("unignore skills-lock.json: differs: .gitignore is not a file\n"),
        "{text}"
    );

    let dir = setup("gi-check-binary", &format!("{STEP}{UNIGNORE}"));
    common::write_bytes(&dir, ".gitignore", &[0xff, 0xfe, 0x00]);
    let text = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        text.contains("gitignore .env: differs: cannot read .gitignore: "),
        "{text}"
    );
    assert!(
        text.contains("unignore skills-lock.json: differs: cannot read .gitignore: "),
        "{text}"
    );
}

#[test]
fn check_goes_through_the_lines_from_the_root_of_the_project() {
    let dir = setup("gi-check-root", STEP);
    write(&dir, ".gitignore", ".env\n!.env.schema\n.tmp/\n");
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["check", "mine"]));

    assert!(text.ends_with("mine: 3 checked, 0 differ\n"), "{text}");
}
