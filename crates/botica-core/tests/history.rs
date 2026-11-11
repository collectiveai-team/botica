//! The writer of the history of findings (ADR 0019).

#![cfg(feature = "fs")]

use botica_core::fingerprint::FingerprintKey;
use botica_core::history::{self, FindingLine, RunLine};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

fn root(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("history")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the root");
    dir
}

fn run(run_id: &str) -> RunLine {
    RunLine {
        v: history::VERSION,
        run_id: run_id.to_owned(),
        started_at: 1,
        commit: None,
        head: None,
        branch: None,
        botica: "0.0.0".to_owned(),
        rules_fingerprint: "0".repeat(64),
        key_id: None,
        mode: "full".to_owned(),
        origin: "cli".to_owned(),
        rules: BTreeMap::new(),
    }
}

fn finding(path: &str, line: usize, fingerprint: &str) -> FindingLine {
    FindingLine {
        v: history::VERSION,
        run_id: "r".to_owned(),
        code: None,
        slug: "a-rule".to_owned(),
        severity: "warning".to_owned(),
        path: path.to_owned(),
        line,
        fingerprint: fingerprint.to_owned(),
        occurrence: 0,
    }
}

#[test]
fn the_occurrence_counts_the_findings_with_one_fingerprint_in_order_of_line() {
    let mut findings = vec![
        finding("a.py", 9, "same"),
        finding("a.py", 3, "same"),
        finding("a.py", 5, "other"),
    ];

    history::number_occurrences(&mut findings);

    let got: Vec<(usize, &str, usize)> = findings
        .iter()
        .map(|f| (f.line, f.fingerprint.as_str(), f.occurrence))
        .collect();
    assert_eq!(got, [(3, "same", 0), (5, "other", 0), (9, "same", 1)]);
}

#[test]
fn append_writes_the_findings_and_then_the_run_one_line_each() {
    let dir = root("append");

    history::append(&dir, &run("r"), &[finding("a.py", 1, "f")]).expect("written");

    let base = dir.join(".botica").join("history");
    let runs = fs::read_to_string(base.join("runs.jsonl")).expect("the runs");
    let findings = fs::read_to_string(base.join("findings.jsonl")).expect("the findings");
    assert_eq!(runs.lines().count(), 1);
    assert!(runs.starts_with("{\"v\":1,\"run_id\":\"r\""), "{runs}");
    assert!(runs.ends_with("}\n"), "{runs}");
    assert_eq!(
        findings,
        "{\"v\":1,\"run_id\":\"r\",\"code\":null,\"slug\":\"a-rule\",\"severity\":\"warning\",\
         \"path\":\"a.py\",\"line\":1,\"fingerprint\":\"f\",\"occurrence\":0}\n"
    );
}

#[test]
fn a_run_with_no_finding_writes_no_findings_file() {
    let dir = root("no-finding");

    history::append(&dir, &run("r"), &[]).expect("written");

    let base = dir.join(".botica").join("history");
    assert!(base.join("runs.jsonl").is_file());
    assert!(!base.join("findings.jsonl").exists());
}

#[test]
fn a_gitignore_that_holds_something_else_is_left_as_it_is() {
    let dir = root("own-gitignore");
    let base = dir.join(".botica").join("history");
    fs::create_dir_all(&base).expect("create the history");
    fs::write(base.join(".gitignore"), "runs.jsonl\n").expect("write it");

    history::append(&dir, &run("r"), &[]).expect("written");

    assert_eq!(
        fs::read_to_string(base.join(".gitignore")).expect("read it"),
        "runs.jsonl\n"
    );
}

#[test]
fn a_botica_that_is_a_file_is_refused() {
    let dir = root("botica-file");
    fs::write(dir.join(".botica"), "").expect("write it");

    let error = history::append(&dir, &run("r"), &[]).expect_err("refused");

    assert!(error.to_string().contains("not a directory"), "{error:#}");
}

#[test]
fn two_run_ids_of_one_instant_differ() {
    let instant = UNIX_EPOCH + Duration::from_secs(1_800_000_000);

    let first = history::run_id(instant);
    let second = history::run_id(instant);

    assert_ne!(first, second);
    assert_eq!(first.len(), 16);
    assert!(
        first.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{first}"
    );
}

#[test]
fn the_id_of_a_key_is_8_hex_digits_of_the_hash_of_the_derived_key() {
    let key = FingerprintKey::new(Some("0123456789abcdef0123456789abcdef")).expect("a key");
    let other = FingerprintKey::new(Some("fedcba9876543210fedcba9876543210")).expect("a key");

    let id = key.id();

    assert_eq!(id.len(), 8);
    assert!(id.bytes().all(|byte| byte.is_ascii_hexdigit()), "{id}");
    assert_eq!(id, key.id());
    assert_ne!(id, other.id());
    assert!(!id.contains("01234567"), "{id}");
}
