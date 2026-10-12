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
    command(&sub, &["lint", ".", "--no-astgrep", "--cache"]);

    assert!(dir.join(".botica/cache/lint-cache.json").exists());
    assert!(!sub.join(".botica").exists());
}

#[test]
fn a_second_run_with_cache_reads_the_results_of_the_first() {
    let dir = project("second-run");
    let run = || {
        let output = command(&dir, &["lint", ".", "--no-astgrep", "--cache", "--timings"]);
        stderr(&output)
    };

    let first = run();
    assert!(first.contains("cache: 0 hits"), "{first}");
    let second = run();
    assert!(second.contains(", 0 misses"), "{second}");
    assert!(!second.contains("cache: 0 hits"), "{second}");
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

/// A file where the cache directory should go: it cannot be created, even by root.
fn blocked_cache(dir: &Path) -> String {
    write(
        dir,
        "blocker",
        "a file where the cache directory should go\n",
    );
    dir.join("blocker/cache")
        .to_str()
        .expect("UTF-8 path")
        .to_owned()
}

#[test]
fn a_cache_that_cannot_be_saved_does_not_hide_the_findings() {
    let dir = project("unsaved-findings");
    write(&dir, "utils.py", "x = 1\n");
    let cache = blocked_cache(&dir);

    let output = command(&dir, &["lint", ".", "--no-astgrep", "--cache-dir", &cache]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(stdout(&output).contains("utils.py"), "{}", stdout(&output));
}

#[test]
fn a_cache_that_cannot_be_saved_is_a_warning_and_not_a_failure() {
    let dir = project("unsaved-clean");
    let cache = blocked_cache(&dir);

    let output = command(&dir, &["lint", ".", "--no-astgrep", "--cache-dir", &cache]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let warning = stderr(&output);
    assert!(warning.contains("not saved"), "{warning}");
    assert!(warning.contains("blocker"), "{warning}");
}
