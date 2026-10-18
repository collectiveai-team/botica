//! `botica recipe install` adds the files of a recipe and never edits one that exists.

mod common;

use std::fs;
use std::path::Path;

use common::recipes::{add_recipe, project, recipe, template};
use common::refs::{failed, ok};
use common::write;

const FILE: &str = "[[files]]\npath = \"prek.toml\"\ntemplate = \"files/prek.toml\"\n";

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).expect("read the file")
}

#[test]
fn a_file_that_does_not_exist_is_written_from_its_template() {
    let dir = project("install-file-add", None);
    add_recipe(&dir, "mine", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "file prek.toml: added\nmine: 1 added, 0 ok, 0 pending\n"
    );
    assert_eq!(read(&dir, "prek.toml"), "repos = []\n");
}

#[test]
fn the_folders_of_the_path_are_created() {
    let dir = project("install-file-folders", None);
    add_recipe(
        &dir,
        "mine",
        "[[files]]\npath = \"a/b/c.toml\"\ntemplate = \"files/c.toml\"\n",
    );
    template(&dir, "files/c.toml", "c = 1\n");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir, "a/b/c.toml"), "c = 1\n");
}

#[test]
fn a_file_that_exists_with_other_text_is_pending_and_stays_as_it_is() {
    let dir = project("install-file-pending", None);
    add_recipe(&dir, "mine", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");
    write(&dir, "prek.toml", "mine = true\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "file prek.toml: pending: exists and differs from the template `files/prek.toml` of `mine`\n\
         mine: 0 added, 0 ok, 1 pending\n"
    );
    assert_eq!(read(&dir, "prek.toml"), "mine = true\n");
}

#[test]
fn a_file_that_has_the_text_of_the_template_is_ok() {
    let dir = project("install-file-same", None);
    add_recipe(&dir, "mine", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");
    write(&dir, "prek.toml", "repos = []\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(text, "file prek.toml: ok\nmine: 0 added, 1 ok, 0 pending\n");
}

#[test]
fn a_file_that_differs_from_the_template_only_in_white_space_is_pending() {
    let dir = project("install-file-white", None);
    add_recipe(&dir, "mine", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");
    write(&dir, "prek.toml", "repos = []\n\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("file prek.toml: pending: "), "{text}");
}

#[cfg(unix)]
#[test]
fn a_link_to_nothing_is_pending_and_nothing_is_written_through_it() {
    let dir = project("install-file-link", None);
    add_recipe(&dir, "mine", FILE);
    std::os::unix::fs::symlink("missing.toml", dir.join("prek.toml")).expect("link");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("file prek.toml: pending: "), "{text}");
    assert!(!dir.join("missing.toml").exists());
}

#[test]
fn a_path_that_is_a_directory_is_pending() {
    let dir = project("install-file-dir", None);
    add_recipe(&dir, "mine", FILE);
    write(&dir, "prek.toml/inside", "x");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.contains("file prek.toml: pending: exists and differs"),
        "{text}"
    );
    assert_eq!(read(&dir, "prek.toml/inside"), "x");
}

#[test]
fn installing_twice_changes_nothing_the_second_time() {
    let dir = project("install-twice", None);
    add_recipe(&dir, "mine", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");
    ok(&recipe(&dir, &["install", "mine"]));

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(text, "file prek.toml: ok\nmine: 0 added, 1 ok, 0 pending\n");
    assert_eq!(read(&dir, "prek.toml"), "repos = []\n");
}

#[test]
fn a_recipe_without_files_or_sections_does_nothing() {
    let dir = project("install-nothing", None);
    add_recipe(&dir, "mine", "rules = [\"file-size-guard\"]\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(text, "mine: 0 added, 0 ok, 0 pending\n");
}

#[test]
fn the_files_of_what_a_recipe_extends_are_added() {
    let dir = project("install-extends", None);
    add_recipe(&dir, "parent", FILE);
    add_recipe(&dir, "child", "extends = [\"parent\"]\n");
    template(&dir, "files/prek.toml", "repos = []\n");

    let text = ok(&recipe(&dir, &["install", "child"]));

    assert_eq!(
        text,
        "file prek.toml: added\nchild: 1 added, 0 ok, 0 pending\n"
    );
}

#[test]
fn several_recipes_are_installed_in_order_and_a_shared_step_once() {
    let dir = project("install-several", None);
    add_recipe(&dir, "parent", FILE);
    add_recipe(
        &dir,
        "child",
        "extends = [\"parent\"]\n[[files]]\npath = \"other.toml\"\ntemplate = \"files/other.toml\"\n",
    );
    template(&dir, "files/prek.toml", "repos = []\n");
    template(&dir, "files/other.toml", "o = 1\n");

    let text = ok(&recipe(&dir, &["install", "parent", "child"]));

    assert_eq!(
        text,
        "file prek.toml: added\nfile other.toml: added\nparent, child: 2 added, 0 ok, 0 pending\n"
    );
}

#[test]
fn a_recipe_that_does_not_exist_is_an_error_and_nothing_is_written() {
    let dir = project("install-unknown", None);
    add_recipe(&dir, "mine", FILE);

    let text = failed(&recipe(&dir, &["install", "mine", "nope"]), 2);

    assert!(text.contains("no recipe `nope`"), "{text}");
    assert!(!dir.join("prek.toml").exists());
}

#[test]
fn a_second_step_with_the_same_path_and_other_text_is_pending() {
    let dir = project("install-same-path", None);
    add_recipe(&dir, "parent", FILE);
    add_recipe(
        &dir,
        "mine",
        "extends = [\"parent\"]\n[[files]]\npath = \"prek.toml\"\ntemplate = \"files/second.toml\"\n",
    );
    template(&dir, "files/prek.toml", "first\n");
    template(&dir, "files/second.toml", "second\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.starts_with("file prek.toml: added\nfile prek.toml: pending: "),
        "{text}"
    );
    assert_eq!(read(&dir, "prek.toml"), "first\n");
}

#[test]
fn the_paths_are_from_the_root_of_the_project() {
    let dir = project("install-root", None);
    add_recipe(&dir, "mine", FILE);
    write(&dir, "deep/er/keep", "");

    ok(&recipe(&dir.join("deep/er"), &["install", "mine"]));

    assert!(dir.join("prek.toml").is_file());
    assert!(!dir.join("deep/er/prek.toml").exists());
}

#[test]
fn the_built_in_base_writes_its_prek_toml_and_check_finds_it() {
    let dir = project("install-base", None);

    ok(&recipe(&dir, &["install", "base"]));

    let template = include_str!("../../../recipes/files/prek-base.toml");
    assert_eq!(read(&dir, "prek.toml"), template);
    let check = failed(
        &recipe(&dir, &["check", "base", "--select", "file-size-guard"]),
        1,
    );
    assert!(check.contains("file prek.toml: ok\n"), "{check}");
}

#[test]
fn a_warning_about_a_skill_that_changes_source_goes_to_stderr() {
    let dir = project("install-warning", None);
    add_recipe(
        &dir,
        "parent",
        "[[skills]]\nsource = \"a/one\"\nnames = [\"tdd\"]\n",
    );
    add_recipe(
        &dir,
        "child",
        "extends = [\"parent\"]\n[[skills]]\nsource = \"b/two\"\nnames = [\"tdd\"]\n",
    );

    let output = recipe(&dir, &["install", "child"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: the skill `tdd` comes from `b/two`"),
        "{stderr}"
    );
}
