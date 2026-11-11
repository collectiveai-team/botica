//! `botica history report`: what the reader skips, counts and escapes (ADR 0019 decisions 2 and
//! 9.4).

mod common;

use common::history::{
    dir, finding, history_dir, report, section, write_downloaded, write_local, RunSpec, HASH,
};
use common::{stderr, stdout};
use serde_json::{json, Value};
use std::fs;

fn append(dir: &std::path::Path, file: &str, text: &str) {
    let path = history_dir(dir).join(file);
    let mut found = fs::read_to_string(&path).unwrap_or_default();
    found.push_str(text);
    fs::create_dir_all(path.parent().expect("a parent")).expect("create the history");
    fs::write(path, found).expect("write the file");
}

fn skipped(text: &str) -> String {
    section(text, "Runs read")
}

#[test]
fn a_line_that_fails_a_check_is_skipped_and_counted() {
    let dir = dir("bad-lines");
    let good = RunSpec::new(1, "cli", "full").rule("no-utils", 0, None);
    write_local(&dir, &[good], &[]);
    let mut bad: Vec<Value> = Vec::new();
    let run = RunSpec::new(2, "cli", "full").rule("bad", 0, None).line();
    for (key, value) in [
        ("v", json!(2)),
        ("run_id", json!("xyz")),
        ("started_at", json!(-1)),
        ("commit", json!("abc")),
        ("key_id", json!("123")),
        ("mode", json!("Full")),
        ("origin", json!("a".repeat(65))),
        ("rules_fingerprint", json!("00")),
        ("detail", json!("text")),
        (
            "rules",
            json!({"x": {"hits": -1, "suppressed": 0, "baselined": 0, "hash": null}}),
        ),
        (
            "rules",
            json!({"x": {"hits": 0, "suppressed": 0, "baselined": 0, "hash": "zz"}}),
        ),
        (
            "rules",
            json!({"a\nb": {"hits": 0, "suppressed": 0, "baselined": 0, "hash": null}}),
        ),
    ] {
        let mut broken = run.clone();
        broken[key] = value;
        bad.push(broken);
    }
    let text: String = bad.iter().map(|line| format!("{line}\n")).collect();
    append(&dir, "runs.jsonl", &text);
    append(&dir, "runs.jsonl", "not json\n[1]\n\n");

    let output = report(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        skipped(&text).contains("14 lines failed their checks"),
        "{text}"
    );
    assert!(
        section(&text, "Rules with no findings").contains("no-utils"),
        "{text}"
    );
    assert!(!text.contains("bad:"), "{text}");
}

#[test]
fn a_finding_line_with_a_path_that_leaves_the_project_or_an_unknown_severity_is_skipped() {
    let dir = dir("bad-findings");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("no-utils", 1, None)],
        &[],
    );
    let mut lines = String::new();
    for (key, value) in [
        ("path", json!("../outside.py")),
        ("path", json!("/etc/passwd")),
        ("path", json!("a\u{7}.py")),
        ("severity", json!("fatal")),
        ("refutation", json!("maybe")),
        ("fingerprint", json!("short")),
        ("occurrence", json!(-1)),
    ] {
        let mut line = finding(1, "no-utils", 1);
        line[key] = value;
        lines.push_str(&format!("{line}\n"));
    }
    append(&dir, "findings.jsonl", &lines);

    let text = stdout(&report(&dir, &[]));

    assert!(
        skipped(&text).contains("7 lines failed their checks"),
        "{text}"
    );
}

#[test]
fn a_key_that_the_reader_does_not_know_is_ignored() {
    let dir = dir("new-key");
    let mut run = RunSpec::new(1, "cli", "full")
        .rule("no-utils", 0, None)
        .line();
    run["added_later"] = json!({"anything": true});
    append(&dir, "runs.jsonl", &format!("{run}\n"));

    let text = stdout(&report(&dir, &[]));

    assert!(
        section(&text, "Rules with no findings").contains("no-utils"),
        "{text}"
    );
    assert!(!skipped(&text).contains("failed their checks"), "{text}");
}

#[test]
fn a_run_of_a_mode_or_an_origin_that_the_reader_does_not_know_enters_no_reading() {
    let dir = dir("unknown");
    let runs = [
        RunSpec::new(1, "cli", "future-mode").rule("only-in-unknown-mode", 0, None),
        RunSpec::new(2, "elsewhere", "full").rule("only-in-unknown-origin", 0, None),
        RunSpec::new(3, "cli", "full").rule("known", 0, None),
    ];
    write_local(&dir, &runs, &[]);

    let text = stdout(&report(&dir, &[]));

    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("known"), "{text}");
    assert!(!quiet.contains("only-in"), "{text}");
    let read = skipped(&text);
    assert!(read.contains("1 runs of a mode or origin that this report does not know (mode future-mode, origin cli)"), "{text}");
    assert!(read.contains("(mode full, origin elsewhere)"), "{text}");
}

#[test]
fn a_finding_whose_run_has_no_run_line_is_ignored() {
    let dir = dir("orphan");
    let hook = RunSpec::new(1, "hook", "staged").rule("no-utils", 0, None);
    write_local(&dir, &[hook], &[finding(99, "no-utils", 1)]);

    let text = stdout(&report(&dir, &[]));

    assert!(!skipped(&text).contains("failed their checks"), "{text}");
}

#[test]
fn a_run_line_that_is_repeated_is_read_once_and_two_different_lines_of_one_name_are_both_ignored() {
    let dir = dir("duplicates");
    let same = RunSpec::new(1, "cli", "full").rule("twice", 0, None).line();
    let one = RunSpec::new(2, "cli", "full")
        .rule("first-version", 0, None)
        .line();
    let mut other = one.clone();
    other["rules"] =
        json!({"second-version": {"hits": 0, "suppressed": 0, "baselined": 0, "hash": null}});
    append(
        &dir,
        "runs.jsonl",
        &format!("{same}\n{same}\n{one}\n{other}\n"),
    );

    let text = stdout(&report(&dir, &[]));

    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("twice: no finding in 1 runs"), "{text}");
    assert!(!quiet.contains("version"), "{text}");
    assert!(skipped(&text).contains("both are ignored"), "{text}");
}

#[test]
fn a_file_with_no_final_newline_does_not_glue_its_last_line_to_another_file() {
    let dir = dir("no-newline");
    let line = RunSpec::new(1, "cli", "full")
        .rule("no-utils", 0, None)
        .line();
    append(&dir, "runs.jsonl", &line.to_string());

    let text = stdout(&report(&dir, &[]));

    assert!(
        section(&text, "Rules with no findings").contains("no-utils"),
        "{text}"
    );
}

#[test]
fn a_run_of_a_role_feeds_the_noise_of_the_role_and_not_the_rules_with_no_findings() {
    let dir = dir("roles");
    let run = RunSpec::new(1, "review:security", "reviewed").rule("review:security", 3, None);
    let mut dismissed = finding(1, "review:security", 2);
    dismissed["refutation"] = json!("not-reproduced");
    let mut confirmed = finding(1, "review:security", 1);
    confirmed["refutation"] = json!("confirmed");
    let mut unjudged = finding(1, "review:security", 3);
    unjudged["refutation"] = Value::Null;
    write_local(&dir, &[run], &[confirmed, dismissed, unjudged]);

    let text = stdout(&report(&dir, &[]));

    let roles = section(&text, "Findings of the roles that refute did not confirm");
    assert!(
        roles.contains("review:security: 1 of 2 judged findings, 50%"),
        "{text}"
    );
    assert!(
        !section(&text, "Rules with no findings").contains("review:security"),
        "{text}"
    );
}

#[test]
fn markdown_puts_a_value_of_the_history_in_code_so_that_it_is_not_read_as_markup() {
    let dir = dir("escape");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("x`](http://evil.example)<b>", 0, None)],
        &[],
    );

    let text = stdout(&report(&dir, &["--format", "markdown"]));

    assert!(
        text.contains("- `x'](http://evil.example)<b>`: no finding in 1 runs"),
        "{text}"
    );
}

#[test]
fn inside_github_actions_the_report_is_printed_between_stop_commands_and_the_token_that_resumes_them(
) {
    let dir = dir("actions");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("::error::forged", 0, None)],
        &[],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["history", "report"])
        .current_dir(&dir)
        .env("GITHUB_ACTIONS", "true")
        .output()
        .expect("run botica");

    let text = stdout(&output);
    let first = text.lines().next().expect("a first line");
    let token = first
        .strip_prefix("::stop-commands::")
        .expect("the stop line");
    assert_eq!(token.len(), 32, "{first}");
    assert_eq!(
        text.lines().last(),
        Some(format!("::{token}::").as_str()),
        "{text}"
    );
    let forged = text.find("::error::forged").expect("the value is printed");
    assert!(
        forged > first.len()
            && forged
                < text
                    .rfind(&format!("::{token}::"))
                    .expect("the resume line")
    );
}

#[test]
fn a_folder_of_a_downloaded_run_that_is_a_symbolic_link_is_not_followed() {
    let dir = dir("symlink");
    let downloads = dir.join("downloads");
    fs::create_dir_all(&downloads).expect("create downloads");
    let outside = dir.join("outside");
    common::history::write_in(
        &outside,
        &[RunSpec::new(1, "ci", "full").rule("linked", 0, None)],
        &[],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, downloads.join("100")).expect("link the folder");

        let text = stdout(&report(
            &dir,
            &["--from", downloads.to_str().expect("UTF-8")],
        ));

        assert!(!text.contains("linked"), "{text}");
    }
}

#[test]
fn a_file_of_the_history_that_is_a_symbolic_link_is_skipped_and_named() {
    let dir = dir("symlink-file");
    let folder = history_dir(&dir);
    fs::create_dir_all(&folder).expect("create the history");
    let target = dir.join("elsewhere.jsonl");
    let line = RunSpec::new(1, "cli", "full")
        .rule("linked-file", 0, None)
        .line();
    fs::write(&target, format!("{line}\n")).expect("write the target");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, folder.join("runs.jsonl")).expect("link the file");

        let text = stdout(&report(&dir, &[]));

        assert!(!text.contains("linked-file"), "{text}");
        assert!(text.contains("is not a regular file: skipped"), "{text}");
    }
}

#[test]
fn a_run_of_the_classifier_enters_no_rule_reading() {
    let dir = dir("classify");
    write_local(
        &dir,
        &[RunSpec::new(1, "classify", "classify").rule("only-classifier", 0, None)],
        &[],
    );

    let text = stdout(&report(&dir, &[]));

    assert!(!text.contains("only-classifier"), "{text}");
}

#[test]
fn an_orphan_finding_does_not_make_a_hook_finding() {
    let dir = dir("orphan-pair");
    write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit(COMMIT_A)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(99, "no-utils", 4)],
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
    let downloads = dir.join("downloads");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        section(&text, "Caught before the commit").contains("No finding."),
        "{text}"
    );
}

const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
