mod common;

use common::{command, project_in, stdout, write};
use std::path::PathBuf;

fn project(name: &str) -> PathBuf {
    let dir = project_in("repo-shape", name, None);
    std::fs::canonicalize(dir).expect("the project exists")
}

/// The paths the run reports for `rule`, in the order of the output.
fn reported(text: &str, rule: &str) -> Vec<String> {
    let tag = format!("[{rule}]");
    text.lines()
        .filter(|line| line.contains(&tag))
        .filter_map(|line| line.split(':').next())
        .map(str::to_owned)
        .collect()
}

fn lint(dir: &std::path::Path) -> String {
    stdout(&command(dir, &["lint", ".", "--no-astgrep"]))
}

#[test]
fn a_single_package_next_to_its_manifest_is_found_by_its_init_file() {
    let dir = project("flat");
    write(&dir, "pyproject.toml", "[project]\nname = \"app\"\n");
    write(&dir, "myapp/__init__.py", "");
    write(&dir, "myapp/models/data/rows.csv", "a,b\n");
    write(&dir, "data/rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "repo-shape"),
        ["myapp/models/data/rows.csv"],
        "{text}"
    );
}

#[test]
fn a_tests_directory_is_not_a_package_even_with_an_init_file() {
    let dir = project("tests-dir");
    write(&dir, "tests/__init__.py", "");
    write(&dir, "tests/data/rows.csv", "a,b\n");
    write(&dir, "test/__init__.py", "");
    write(&dir, "test/reports/q1.txt", "r\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "repo-shape"),
        Vec::<String>::new(),
        "{text}"
    );
}

#[test]
fn a_src_directory_is_not_a_package() {
    let dir = project("src");
    write(&dir, "src/__init__.py", "");
    write(&dir, "src/data/rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "repo-shape"),
        Vec::<String>::new(),
        "{text}"
    );
}

/// The `packages:` line of `botica explain repo-shape`.
fn explained_packages(dir: &std::path::Path) -> String {
    let output = command(dir, &["explain", "repo-shape"]);
    let text = stdout(&output);
    text.lines()
        .find(|line| line.starts_with("packages:"))
        .unwrap_or_else(|| panic!("explain has no `packages:` line:\n{text}"))
        .to_owned()
}

#[test]
fn explain_lists_the_packages_it_detected() {
    let dir = project("explain-detected");
    write(&dir, "backend/pyproject.toml", "[project]\nname = \"b\"\n");
    write(&dir, "frontend/package.json", "{}\n");

    assert_eq!(
        explained_packages(&dir),
        "packages:  backend, frontend (detected)"
    );
}

#[test]
fn explain_says_the_rule_does_nothing_when_there_is_no_package() {
    let dir = project("explain-none");
    write(&dir, "data/rows.csv", "a,b\n");

    let line = explained_packages(&dir);

    assert!(line.starts_with("packages:  none (detected)"), "{line}");
    assert!(line.contains("does nothing"), "{line}");
}

#[test]
fn explain_names_the_configuration_as_the_source_of_the_packages() {
    let dir = project("explain-configured");
    write(&dir, "botica.toml", "packages = [\"lib\"]\n");

    assert_eq!(explained_packages(&dir), "packages:  lib (configuration)");
}

/// A project whose root holds `botica.toml` and two packages, each with a `data/` directory.
fn monorepo(name: &str) -> PathBuf {
    let dir = project(name);
    write(&dir, "botica.toml", "");
    write(&dir, "backend/pyproject.toml", "[project]\nname = \"b\"\n");
    write(&dir, "backend/data/rows.csv", "a,b\n");
    write(&dir, "backend/app/data/more.csv", "a,b\n");
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/data/rows.csv", "a,b\n");
    dir
}

fn lint_from(dir: &std::path::Path, below: &str) -> String {
    stdout(&command(&dir.join(below), &["lint", ".", "--no-astgrep"]))
}

#[test]
fn run_from_inside_a_package_every_artifact_directory_is_inside_it() {
    let dir = monorepo("from-package");

    let text = lint_from(&dir, "backend");

    assert_eq!(
        reported(&text, "repo-shape"),
        ["app/data/more.csv", "data/rows.csv"],
        "{text}"
    );
}

#[test]
fn run_from_deeper_inside_a_package_the_whole_tree_is_still_inside_it() {
    let dir = monorepo("from-deep");

    let text = lint_from(&dir, "backend/app");

    assert_eq!(reported(&text, "repo-shape"), ["data/more.csv"], "{text}");
}

#[test]
fn run_from_a_directory_that_holds_packages_their_paths_are_found_below_it() {
    let dir = monorepo("from-above");
    write(&dir, "tools/backend/data/rows.csv", "a,b\n");
    write(&dir, "botica.toml", "packages = [\"tools/backend\"]\n");

    let text = lint_from(&dir, "tools");

    assert_eq!(
        reported(&text, "repo-shape"),
        ["backend/data/rows.csv"],
        "{text}"
    );
}

#[test]
fn run_from_a_directory_outside_every_package_nothing_is_reported() {
    let dir = monorepo("from-outside");
    write(&dir, "docs/data/rows.csv", "a,b\n");

    let text = lint_from(&dir, "docs");

    assert_eq!(
        reported(&text, "repo-shape"),
        Vec::<String>::new(),
        "{text}"
    );
}

#[test]
fn the_packages_of_the_configuration_replace_the_detection() {
    let dir = project("configured");
    write(&dir, "botica.toml", "packages = [\"lib\"]\n");
    write(&dir, "lib/data/rows.csv", "a,b\n");
    write(
        &dir,
        "backend/pyproject.toml",
        "[project]\nname = \"backend\"\n",
    );
    write(&dir, "backend/data/rows.csv", "a,b\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "repo-shape"),
        ["lib/data/rows.csv"],
        "{text}"
    );
}

#[test]
fn an_empty_list_of_packages_means_there_are_none() {
    let dir = project("none-configured");
    write(&dir, "botica.toml", "packages = []\n");
    write(
        &dir,
        "backend/pyproject.toml",
        "[project]\nname = \"backend\"\n",
    );
    write(&dir, "backend/data/rows.csv", "a,b\n");

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_data_directory_inside_a_package_is_reported_and_the_one_at_the_root_is_not() {
    let dir = project("data-in-package");
    write(
        &dir,
        "backend/pyproject.toml",
        "[project]\nname = \"backend\"\n",
    );
    write(&dir, "backend/data/rows.csv", "a,b\n");
    write(&dir, "data/rows.csv", "a,b\n");

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    let text = stdout(&output);
    assert_eq!(
        reported(&text, "repo-shape"),
        ["backend/data/rows.csv"],
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
}
