//! `botica history report`: "Caught before the commit" and "Reached CI" (ADR 0019 decision 8),
//! with the pairing of a hook run and a CI run by branch and ancestry.

mod common;

use common::history::{
    dir, finding, report, section, write_downloaded, write_local, RunSpec, HASH,
};
use common::stdout;
use std::path::Path;

const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// The report of `dir` with the CI runs that were downloaded into `dir/downloads`.
fn joined(dir: &Path) -> String {
    let downloads = dir.join("downloads");
    stdout(&report(
        dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ))
}

/// A hook run of mode `staged` on `topic`, then a CI run of mode `diff`, that hold the findings
/// whose fingerprints are `hook` and `ci`.
fn hook_then_ci(name: &str, hook: &[u64], ci: &[u64]) -> String {
    let dir = dir(name);
    let hook_run = RunSpec::new(1, "hook", "staged")
        .ago(7200)
        .commit(COMMIT_A)
        .rule("no-utils", 0, Some(HASH));
    let findings: Vec<_> = hook
        .iter()
        .map(|print| finding(1, "no-utils", *print))
        .collect();
    write_local(&dir, &[hook_run], &findings);
    let ci_run = RunSpec::new(2, "ci", "diff")
        .ago(3600)
        .commit(COMMIT_B)
        .head(COMMIT_B)
        .rule("no-utils", 0, Some(HASH));
    let findings: Vec<_> = ci
        .iter()
        .map(|print| finding(2, "no-utils", *print))
        .collect();
    write_downloaded(&dir.join("downloads"), "100", true, &[ci_run], &findings);
    joined(&dir)
}

#[test]
fn a_finding_of_a_hook_run_that_the_paired_ci_run_does_not_hold_was_caught_before_the_commit() {
    let text = hook_then_ci("caught", &[1, 2], &[2]);

    let caught = section(&text, "Caught before the commit");
    assert!(
        caught.contains("1 of 1 hook runs have a CI run to pair with."),
        "{text}"
    );
    assert!(caught.contains("no-utils: 1"), "{text}");
    let reached = section(&text, "Reached CI");
    assert!(reached.contains("No finding."), "{text}");
}

#[test]
fn a_finding_of_a_ci_run_that_no_hook_run_before_it_held_reached_ci() {
    let text = hook_then_ci("reached", &[1], &[1, 3]);

    let reached = section(&text, "Reached CI");
    assert!(
        reached.contains("1 of 1 CI runs have a hook run before them."),
        "{text}"
    );
    assert!(reached.contains("no-utils: 1"), "{text}");
    assert!(
        section(&text, "Caught before the commit").contains("No finding."),
        "{text}"
    );
}

#[test]
fn a_hook_run_that_the_ci_run_does_not_follow_pairs_with_nothing() {
    let dir = dir("hook-after");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(60)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit")
            .contains("none has a CI run of mode `diff` on its branch after it"),
        "{text}"
    );
}

#[test]
fn a_ci_run_of_the_head_that_the_hook_checked_is_not_paired_with_it() {
    let dir = dir("same-head");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_A)
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
fn runs_of_two_branches_are_not_paired() {
    let dir = dir("branches");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .branch(Some("one"))
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .branch(Some("two"))
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("none has a CI run"),
        "{text}"
    );
    assert!(
        section(&text, "Reached CI").contains("none has a hook run"),
        "{text}"
    );
}

#[test]
fn a_run_with_no_branch_is_not_paired() {
    let dir = dir("no-branch");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .branch(None)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .branch(None)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("No hook run of mode `staged`"),
        "{text}"
    );
    assert!(
        section(&text, "Reached CI").contains("No CI run of mode `diff`"),
        "{text}"
    );
}

#[test]
fn a_hook_run_of_mode_paths_feeds_neither_paired_reading() {
    let dir = dir("paths-hook");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "paths")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(2, "no-utils", 1)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("No hook run of mode `staged`"),
        "{text}"
    );
    assert!(
        section(&text, "Reached CI").contains("none has a hook run"),
        "{text}"
    );
}

#[test]
fn a_hook_run_that_came_with_a_downloaded_artifact_is_not_a_hook_run() {
    let dir = dir("forged-hook");
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[
            RunSpec::new(1, "hook", "staged")
                .ago(7200)
                .commit(COMMIT_A)
                .rule("no-utils", 0, Some(HASH)),
            RunSpec::new(2, "ci", "diff")
                .ago(3600)
                .head(COMMIT_B)
                .rule("no-utils", 0, Some(HASH)),
        ],
        &[finding(2, "no-utils", 1)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Reached CI").contains("none has a hook run"),
        "{text}"
    );
}

#[test]
fn a_run_of_ci_from_a_source_that_is_not_trusted_still_feeds_the_paired_readings() {
    let dir = dir("untrusted-pair");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        false,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("no-utils: 1"),
        "{text}"
    );
}

#[test]
fn the_earliest_ci_run_after_the_hook_run_is_the_one_that_pairs() {
    let dir = dir("earliest");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(9000)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(1, "no-utils", 1)],
    );
    let downloads = dir.join("downloads");
    write_downloaded(
        &downloads,
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(7200)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );
    write_downloaded(
        &downloads,
        "101",
        true,
        &[RunSpec::new(3, "ci", "diff")
            .ago(3600)
            .head("cccccccccccccccccccccccccccccccccccccccc")
            .rule("no-utils", 0, Some(HASH))],
        &[finding(3, "no-utils", 1)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("no-utils: 1"),
        "{text}"
    );
}
