mod common;

use common::{command, lines, project_in, stderr, stdout, write};
use std::path::PathBuf;

fn project(name: &str) -> PathBuf {
    let dir = project_in("review-fixes", name, None);
    std::fs::canonicalize(dir).expect("the project exists")
}

fn count(text: &str, needle: &str) -> usize {
    text.lines().filter(|line| line.contains(needle)).count()
}

#[test]
fn a_file_named_twice_is_reported_once() {
    let dir = project("twice");
    write(&dir, "big.txt", &lines(701));
    for args in [
        &["lint", "big.txt", "big.txt", "--no-astgrep"][..],
        &["lint", "big.txt", "./big.txt", "--no-astgrep"],
        &["lint", ".", "big.txt", "--no-astgrep"],
    ] {
        let text = stdout(&command(&dir, args));
        assert_eq!(count(&text, "[file-size-guard]"), 1, "{args:?}:\n{text}");
    }
}

#[test]
fn a_missing_path_is_reported_as_the_walk_would_name_it() {
    let dir = project("io-paths");
    let absolute = dir.join("gone.py");
    let output = command(
        &dir,
        &[
            "lint",
            "./zz.py",
            absolute.to_str().expect("UTF-8 path"),
            "aa.py",
            "--no-astgrep",
        ],
    );
    let text = stdout(&output);
    let paths: Vec<&str> = text
        .lines()
        .filter_map(|line| line.split(':').next())
        .collect();
    assert_eq!(paths, ["aa.py", "gone.py", "zz.py"], "{text}");
    assert_eq!(output.status.code(), Some(1));
}

const SUPPRESSES_AN_EMBEDDED_RULE: &str = "print('x')  # botica: ignore[log-no-print]\n";

#[test]
fn naming_an_embedded_rule_that_does_not_run_is_not_an_unknown_rule() {
    let dir = project("embedded-off");
    write(&dir, "a.py", SUPPRESSES_AN_EMBEDDED_RULE);
    let without = stdout(&command(&dir, &["lint", ".", "--no-astgrep"]));
    assert!(!without.contains("Unknown rule"), "{without}");
    assert!(without.contains("non-enabled: `log-no-print`"), "{without}");

    let narrow = stdout(&command(
        &dir,
        &[
            "lint",
            ".",
            "--select",
            "invalid-suppression,unused-suppression",
        ],
    ));
    assert!(!narrow.contains("Unknown rule"), "{narrow}");
    assert!(narrow.contains("non-enabled: `log-no-print`"), "{narrow}");
}

#[test]
fn a_name_nobody_knows_is_still_an_unknown_rule() {
    let dir = project("unknown");
    write(&dir, "a.py", "x = 1  # botica: ignore[no-such-rule]\n");
    let text = stdout(&command(&dir, &["lint", ".", "--no-astgrep"]));
    assert!(text.contains("Unknown rule `no-such-rule`"), "{text}");
}

#[test]
fn cached_results_are_not_reused_under_another_scope() {
    let dir = project("cache-scope");
    write(&dir, "big.txt", &lines(701));
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");
    let lint = |scope: &str| {
        command(
            &dir,
            &[
                "lint",
                ".",
                "--no-astgrep",
                "--cache-dir",
                cache,
                "--scope",
                scope,
            ],
        )
    };

    let project_only = lint("project");
    assert_eq!(stdout(&project_only), "", "{}", stderr(&project_only));

    let all = lint("all");
    assert_eq!(
        count(&stdout(&all), "[file-size-guard]"),
        1,
        "{}",
        stdout(&all)
    );
    assert_eq!(all.status.code(), Some(1));
}
