//! `botica history report`: the pairing of a hook run and a CI run by the ancestry of their
//! commits in a real repository (ADR 0019 decision 8).

mod common;

use common::history::{finding, report, section, write_downloaded, write_local, RunSpec, HASH};
use common::record::{git, repository};
use common::stdout;
use std::fs;
use std::path::Path;

/// The report of `dir` with the CI runs that were downloaded into `dir/downloads`.
fn joined(dir: &Path) -> String {
    let downloads = dir.join("downloads");
    stdout(&report(
        dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ))
}
/// A repository on `topic` with two commits, and their names.
fn two_commits(name: &str) -> (std::path::PathBuf, String, String) {
    let dir = repository(name);
    let first = git(&dir, &["rev-parse", "HEAD"]);
    fs::write(dir.join("b.py"), "y = 1\n").expect("write b.py");
    git(&dir, &["add", "b.py"]);
    git(&dir, &["commit", "--quiet", "-m", "second"]);
    let second = git(&dir, &["rev-parse", "HEAD"]);
    (dir, first, second)
}

#[test]
fn a_hook_run_pairs_with_a_ci_run_when_its_commit_is_an_ancestor_of_the_head() {
    let (dir, first, second) = two_commits("history-ancestor");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(60)
            .commit(&first)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(&second)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("no-utils: 1"),
        "the order is the one of the commits, not the one of the clocks: {text}"
    );
}

#[test]
fn a_hook_run_on_the_head_itself_does_not_pair_even_when_its_clock_is_earlier() {
    let (dir, _, second) = two_commits("history-same-commit");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(&second)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(&second)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("none has a CI run"),
        "{text}"
    );
}

#[test]
fn a_ci_run_on_an_older_head_pairs_with_no_hook_run_that_came_after_a_newer_one() {
    let (dir, first, second) = two_commits("history-rerun");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(1000)
            .commit("dddddddddddddddddddddddddddddddddddddddd")
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    let downloads = dir.join("downloads");
    write_downloaded(
        &downloads,
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3000)
            .head(&second)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );
    write_downloaded(
        &downloads,
        "101",
        true,
        &[RunSpec::new(3, "ci", "diff")
            .ago(500)
            .head(&first)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    let caught = section(&text, "Caught before the commit");
    assert!(caught.contains("none has a CI run"), "{text}");
}
