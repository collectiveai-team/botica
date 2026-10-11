//! Where the result cache lives and what it leaves around.

mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::{Path, PathBuf};

fn project(name: &str) -> PathBuf {
    let dir = project_in("cache-dir", name, Some(""));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    write(&dir, "app.py", "x = 1\n");
    dir
}

fn lint_with(dir: &Path, extra: &[&str]) -> String {
    let mut args = vec!["lint", ".", "--no-astgrep"];
    args.extend(extra);
    stdout(&command(dir, &args))
}

#[test]
fn a_cache_directory_that_botica_creates_ignores_itself() {
    let dir = project("self-ignore");
    let cache = dir.join("out/cache");

    lint_with(&dir, &["--cache-dir", cache.to_str().expect("UTF-8 path")]);

    assert_eq!(
        std::fs::read_to_string(cache.join(".gitignore")).expect("the cache ignores itself"),
        "*\n"
    );
}

#[test]
fn cache_alone_keeps_the_results_in_dot_botica_under_the_root_of_the_project() {
    let dir = project("default-place");
    write(&dir, "sub/other.py", "y = 2\n");

    // Run from a subdirectory: the place is under the root, not under where the command ran.
    let sub = dir.join("sub");
    let first = command(&sub, &["lint", ".", "--no-astgrep", "--cache", "--timings"]);
    assert!(dir.join(".botica/cache/lint-cache.json").exists());
    assert!(!sub.join(".botica").exists());
    assert!(
        stderr(&first).contains("0 hits, 1 misses"),
        "{}",
        stderr(&first)
    );

    let second = command(&sub, &["lint", ".", "--no-astgrep", "--cache", "--timings"]);
    assert!(
        stderr(&second).contains("1 hits, 0 misses"),
        "{}",
        stderr(&second)
    );
}

#[test]
fn cache_and_cache_dir_together_are_refused() {
    let dir = project("both");

    let output = command(&dir, &["lint", ".", "--cache", "--cache-dir", "elsewhere"]);

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_directory_that_was_already_there_is_left_as_it_was() {
    let dir = project("existing");

    // `--cache-dir .` would otherwise hide the whole project from git.
    lint_with(&dir, &["--cache-dir", "."]);

    assert!(dir.join("lint-cache.json").exists());
    assert!(!dir.join(".gitignore").exists());
}
