//! `botica history report`: a rule takes part in a pairing only when both runs record the same
//! hash for it (ADR 0019 decision 8).

mod common;

use common::history::{
    dir, finding, report, section, write_downloaded, write_local, RunSpec, HASH, OTHER_HASH,
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

#[test]
fn a_rule_with_no_hash_is_not_comparable_and_the_report_says_so() {
    let dir = dir("null-hash");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, None)],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, None)],
        &[finding(2, "no-utils", 9)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("No finding."),
        "{text}"
    );
    assert!(
        section(&text, "Reached CI").contains("No finding."),
        "{text}"
    );
    let left_out = section(&text, "Rules that are not comparable");
    assert!(
        left_out.contains("no-utils: a run records no hash for it"),
        "{text}"
    );
}

#[test]
fn a_rule_whose_hash_changed_between_the_runs_is_not_comparable() {
    let dir = dir("changed-hash");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff").ago(3600).head(COMMIT_B).rule(
            "no-utils",
            0,
            Some(OTHER_HASH),
        )],
        &[finding(2, "no-utils", 9)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Reached CI").contains("No finding."),
        "{text}"
    );
    let left_out = section(&text, "Rules that are not comparable");
    assert!(
        left_out.contains("no-utils: the hash differs between the runs"),
        "{text}"
    );
}

#[test]
fn a_rule_that_ran_in_only_one_of_the_two_runs_takes_part_in_neither_reading() {
    let dir = dir("absent");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("other", 0, Some(HASH))],
        &[],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(2, "no-utils", 9)],
    );

    let text = joined(&dir);

    assert!(
        section(&text, "Reached CI").contains("No finding."),
        "{text}"
    );
    assert!(!text.contains("Rules that are not comparable"), "{text}");
}
