//! `botica recipe install` adds the section of a recipe to its file when the marker is missing.

mod common;

use std::fs;
use std::path::Path;

use common::recipes::{add_recipe, project, recipe, template};
#[cfg(unix)]
use common::recipes::{with_npx, NPX_INSTALLS};
use common::refs::{failed, ok};
use common::{write, write_bytes};

const SECTION: &str = "[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Standards\"\n\
                       template = \"sections/standards.md\"\n";
const TEXT: &str = "## Standards\n\n- one\n";

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).expect("read the file")
}

fn setup(name: &str) -> std::path::PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "mine", SECTION);
    template(&dir, "sections/standards.md", TEXT);
    dir
}

#[test]
fn a_file_that_does_not_exist_is_written_with_the_section() {
    let dir = setup("install-section-new");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "section AGENTS.md `## Standards`: added: created the file\n\
         mine: 1 added, 0 ok, 0 pending\n"
    );
    assert_eq!(read(&dir, "AGENTS.md"), TEXT);
}

#[test]
fn the_section_is_appended_after_a_blank_line_to_a_file_without_the_marker() {
    let dir = setup("install-section-append");
    write(&dir, "AGENTS.md", "# Notes\n\nkeep this\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.starts_with("section AGENTS.md `## Standards`: added: appended to the file\n"),
        "{text}"
    );
    assert_eq!(
        read(&dir, "AGENTS.md"),
        format!("# Notes\n\nkeep this\n\n{TEXT}")
    );
}

#[test]
fn a_file_that_does_not_end_in_a_newline_gets_one_and_a_blank_line() {
    let dir = setup("install-section-no-newline");
    write(&dir, "AGENTS.md", "# Notes");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir, "AGENTS.md"), format!("# Notes\n\n{TEXT}"));
}

#[test]
fn an_empty_file_gets_the_section_alone() {
    let dir = setup("install-section-empty");
    write(&dir, "AGENTS.md", "");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir, "AGENTS.md"), TEXT);
}

#[test]
fn a_file_that_has_the_marker_is_ok_and_stays_as_it_is() {
    let dir = setup("install-section-present");
    write(&dir, "AGENTS.md", "# Notes\n\n## Standards\n\nmine\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "section AGENTS.md `## Standards`: ok\nmine: 0 added, 1 ok, 0 pending\n"
    );
    assert_eq!(read(&dir, "AGENTS.md"), "# Notes\n\n## Standards\n\nmine\n");
}

#[test]
fn a_path_that_is_a_directory_is_pending() {
    let dir = setup("install-section-dir");
    write(&dir, "AGENTS.md/inside", "x");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: pending: is not a file\n"),
        "{text}"
    );
    assert!(text.ends_with("mine: 0 added, 0 ok, 1 pending\n"), "{text}");
}

#[test]
fn a_file_that_cannot_be_read_is_pending_and_stays_as_it_is() {
    let dir = setup("install-section-unreadable");
    write_bytes(&dir, "AGENTS.md", &[0xff, 0xfe, 0x00]);

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: pending: cannot read the file"),
        "{text}"
    );
    assert_eq!(
        fs::read(dir.join("AGENTS.md")).expect("read"),
        [0xff, 0xfe, 0x00]
    );
}

#[test]
fn installing_twice_adds_the_section_once() {
    let dir = setup("install-section-twice");
    write(&dir, "AGENTS.md", "# Notes\n");
    ok(&recipe(&dir, &["install", "mine"]));

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.starts_with("section AGENTS.md `## Standards`: ok\n"),
        "{text}"
    );
    assert_eq!(read(&dir, "AGENTS.md"), format!("# Notes\n\n{TEXT}"));
}

#[test]
fn two_sections_of_one_file_are_added_in_order() {
    let dir = project("install-section-two", None);
    add_recipe(
        &dir,
        "mine",
        &format!(
            "{SECTION}[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Notes\"\n\
             template = \"sections/notes.md\"\n"
        ),
    );
    template(&dir, "sections/standards.md", TEXT);
    template(&dir, "sections/notes.md", "## Notes\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.ends_with("mine: 2 added, 0 ok, 0 pending\n"), "{text}");
    assert_eq!(read(&dir, "AGENTS.md"), format!("{TEXT}\n## Notes\n"));
}

#[test]
fn files_come_before_sections_in_the_report_and_in_the_writing() {
    let dir = project("install-order", None);
    add_recipe(
        &dir,
        "mine",
        &format!("{SECTION}[[files]]\npath = \"AGENTS.md\"\ntemplate = \"files/agents.md\"\n"),
    );
    template(&dir, "sections/standards.md", TEXT);
    template(&dir, "files/agents.md", "# Agents\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.starts_with(
            "file AGENTS.md: added\nsection AGENTS.md `## Standards`: added: appended"
        ),
        "{text}"
    );
    assert_eq!(read(&dir, "AGENTS.md"), format!("# Agents\n\n{TEXT}"));
}

#[test]
fn the_path_of_a_section_is_from_the_root_of_the_project() {
    let dir = setup("install-section-root");
    write(&dir, "deep/er/keep", "");

    ok(&recipe(&dir.join("deep/er"), &["install", "mine"]));

    assert_eq!(read(&dir, "AGENTS.md"), TEXT);
    assert!(!dir.join("deep/er/AGENTS.md").exists());
}

#[test]
fn a_section_that_two_recipes_share_is_added_once() {
    let dir = project("install-section-shared", None);
    add_recipe(&dir, "parent", SECTION);
    add_recipe(&dir, "child", "extends = [\"parent\"]\n");
    template(&dir, "sections/standards.md", TEXT);

    let text = ok(&recipe(&dir, &["install", "parent", "child"]));

    assert_eq!(
        text,
        "section AGENTS.md `## Standards`: added: created the file\n\
         parent, child: 1 added, 0 ok, 0 pending\n"
    );
}

#[cfg(unix)]
#[test]
fn the_built_in_base_adds_the_standards_index_and_check_finds_it() {
    let dir = project("install-base-section", None);
    write(&dir, "AGENTS.md", "# Agent Notes\n");

    ok(&with_npx(&dir, NPX_INSTALLS, &["install", "base"]));

    let agents = read(&dir, "AGENTS.md");
    let index = include_str!("../../../recipes/sections/standards-index.md");
    assert_eq!(agents, format!("# Agent Notes\n\n{index}"));
    let check = recipe(&dir, &["check", "base", "--select", "file-size-guard"]);
    let check = String::from_utf8_lossy(&check.stdout);
    assert!(
        check.contains("section AGENTS.md `## Engineering Standards`: ok\n"),
        "{check}"
    );
}
