//! `botica recipe install` adds the files of a recipe and never edits one that exists.

mod common;

use std::fs;
use std::path::Path;

use common::recipes::{add_recipe, project, recipe, template};
#[cfg(unix)]
use common::recipes::{with_npx, NPX_INSTALLS};
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

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert_eq!(
        text,
        "file prek.toml: pending: exists and differs from the template `files/prek.toml` of `mine`\n\
         mine: 0 added, 0 ok, 1 pending\n"
    );
    assert_eq!(read(&dir, "prek.toml"), "mine = true\n");
}

#[test]
fn a_pending_step_does_not_stop_the_others_and_the_command_exits_with_1() {
    let dir = project("install-pending-exit", None);
    add_recipe(
        &dir,
        "mine",
        &format!("{FILE}[[files]]\npath = \"other.toml\"\ntemplate = \"files/other.toml\"\n"),
    );
    template(&dir, "files/prek.toml", "repos = []\n");
    template(&dir, "files/other.toml", "o = 1\n");
    write(&dir, "prek.toml", "mine = true\n");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(text.contains("file other.toml: added\n"), "{text}");
    assert!(text.ends_with("mine: 1 added, 0 ok, 1 pending\n"), "{text}");
    assert_eq!(read(&dir, "other.toml"), "o = 1\n");
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

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(text.contains("file prek.toml: pending: "), "{text}");
}

#[cfg(unix)]
#[test]
fn a_link_to_nothing_is_pending_and_nothing_is_written_through_it() {
    let dir = project("install-file-link", None);
    add_recipe(&dir, "mine", FILE);
    std::os::unix::fs::symlink("missing.toml", dir.join("prek.toml")).expect("link");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(text.contains("file prek.toml: pending: "), "{text}");
    assert!(!dir.join("missing.toml").exists());
}

#[test]
fn a_path_that_is_a_directory_is_pending() {
    let dir = project("install-file-dir", None);
    add_recipe(&dir, "mine", FILE);
    write(&dir, "prek.toml/inside", "x");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

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
fn a_recipe_without_rules_or_steps_does_nothing() {
    let dir = project("install-nothing", None);
    add_recipe(&dir, "mine", "description = \"Nothing\"\n");

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
fn a_file_that_two_recipes_that_do_not_extend_each_other_list_is_done_once() {
    let dir = project("install-unrelated-file", None);
    add_recipe(&dir, "one", FILE);
    add_recipe(&dir, "two", FILE);
    template(&dir, "files/prek.toml", "repos = []\n");

    let text = ok(&recipe(&dir, &["install", "one", "two"]));

    assert_eq!(
        text,
        "file prek.toml: added\none, two: 1 added, 0 ok, 0 pending\n"
    );
}

#[test]
fn two_files_that_share_a_template_are_both_added() {
    let dir = project("install-shared-template", None);
    add_recipe(
        &dir,
        "one",
        "[[files]]\npath = \"a.toml\"\ntemplate = \"files/t.toml\"\n\n\
         [[files]]\npath = \"b.toml\"\ntemplate = \"files/t.toml\"\n",
    );
    template(&dir, "files/t.toml", "t = 1\n");

    let text = ok(&recipe(&dir, &["install", "one"]));

    assert_eq!(
        text,
        "file a.toml: added\nfile b.toml: added\none: 2 added, 0 ok, 0 pending\n"
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

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

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

#[cfg(unix)]
#[test]
fn the_built_in_base_writes_its_prek_toml_and_check_finds_it() {
    let dir = project("install-base", None);

    ok(&with_npx(&dir, NPX_INSTALLS, &["install", "base"]));

    let template = include_str!("../../../catalog/recipes/files/prek-base.toml");
    let written = read(&dir, "prek.toml");
    let this_release = format!("rev = \"v{}\"", env!("CARGO_PKG_VERSION"));
    let mirror = "repo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n";
    let rev = written
        .split(mirror)
        .nth(1)
        .and_then(|rest| rest.lines().next());
    assert_eq!(rev, Some(this_release.as_str()), "{written}");
    let template_rev = template
        .split(mirror)
        .nth(1)
        .and_then(|rest| rest.lines().next())
        .expect("the template names the mirror and its rev");
    let template_version = template_rev
        .trim_start_matches("rev = \"v")
        .trim_end_matches('"');
    let package = format!("uvx botica@{} ", env!("CARGO_PKG_VERSION"));
    let expected = template
        .replace(template_rev, &this_release)
        .replace(&format!("uvx botica@{template_version} "), &package);
    assert_eq!(written, expected);
    let workflow = read(&dir, ".github/workflows/abide.yml");
    assert_eq!(workflow.matches(&package).count(), 4, "{workflow}");
    let check = failed(
        &recipe(&dir, &["check", "base", "--select", "file-size-guard"]),
        1,
    );
    assert!(check.contains("file prek.toml: ok\n"), "{check}");
}
