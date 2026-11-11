//! `botica history report`: the cap of the pairing and the rules that it compares (ADR 0019
//! decision 8).

mod common;

use common::history::RunSpec;
use common::history::{dir, finding, report, section, write_downloaded};
use common::stdout;
use std::path::Path;

fn from(dir: &Path) -> String {
    stdout(&report(
        dir,
        &["--from", dir.join("downloads").to_str().expect("UTF-8")],
    ))
}

#[test]
fn the_limit_of_500_runs_counts_each_branch_and_each_kind_of_run_on_its_own() {
    let dir = dir("per-branch");
    let hash = Some(common::history::HASH);
    let mut hooks: Vec<RunSpec> = (1..=501)
        .map(|n| {
            RunSpec::new(n, "hook", "staged")
                .ago(100_000 - n * 10)
                .branch(Some("busy"))
                .rule("no-utils", 0, hash)
        })
        .collect();
    hooks.push(
        RunSpec::new(900, "hook", "staged")
            .ago(50)
            .branch(Some("quiet"))
            .rule("no-utils", 0, hash),
    );
    common::history::write_local(&dir, &hooks, &[]);
    let cis: Vec<RunSpec> = (1..=502)
        .map(|n| {
            RunSpec::new(2000 + n, "ci", "diff")
                .ago(1000 - n)
                .branch(Some("busy"))
                .rule("no-utils", 0, hash)
        })
        .collect();
    write_downloaded(&dir.join("downloads"), "100", true, &cis, &[]);

    let text = from(&dir);

    assert!(
        section(&text, "Pairing limits").contains("3 older runs were left out"),
        "{text}"
    );
    assert!(
        section(&text, "Caught before the commit").contains("of 502 hook runs"),
        "{text}"
    );
}

#[test]
fn a_hook_run_that_holds_a_fingerprint_under_another_hash_does_not_hide_it_from_ci() {
    let dir = dir("other-hash");
    let (one, two) = (common::history::HASH, common::history::OTHER_HASH);
    common::history::write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .rule("no-utils", 0, Some(two))],
        &[finding(1, "no-utils", 4)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .rule("no-utils", 0, Some(one))],
        &[finding(2, "no-utils", 4)],
    );

    let text = from(&dir);

    assert!(
        section(&text, "Reached CI").contains("No finding."),
        "{text}"
    );
    assert!(
        section(&text, "Rules that are not comparable").contains("no-utils: the hash differs"),
        "{text}"
    );
}
