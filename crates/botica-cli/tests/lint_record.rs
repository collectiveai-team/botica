//! `botica lint --record` and `[history] record` (ADR 0019): each run appends its findings to
//! `.botica/history/findings.jsonl` and itself to `runs.jsonl`.

mod common;

use common::record::{findings, history, is_hex, lint, lint_with, project, runs, KEY};
use common::{project_in, stderr, stdout, write};
use serde_json::Value;
use std::fs;

#[test]
fn record_appends_one_run_and_one_line_for_each_finding() {
    let dir = project("one-run", None);

    let output = lint(&dir, &["--record"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let runs = runs(&dir);
    let findings = findings(&dir);
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(findings.len(), 1, "{findings:?}");
    let (run, finding) = (&runs[0], &findings[0]);
    assert_eq!(run["v"], 1);
    assert_eq!(finding["v"], 1);
    assert_eq!(finding["run_id"], run["run_id"]);
    assert_eq!(finding["code"], "BT002");
    assert_eq!(finding["slug"], "unused-suppression");
    assert_eq!(finding["severity"], "warning");
    assert_eq!(finding["path"], "a.py");
    assert_eq!(finding["line"], 2);
    assert_eq!(finding["occurrence"], 0);
    let run_id = run["run_id"].as_str().expect("a run id");
    assert!(is_hex(run_id, 16), "{run_id}");
    let fingerprint = finding["fingerprint"].as_str().expect("a fingerprint");
    assert!(is_hex(fingerprint, 16), "{fingerprint}");
    assert_eq!(run["botica"], env!("CARGO_PKG_VERSION"));
    assert_eq!(run["mode"], "full");
    assert_eq!(run["origin"], "cli");
    assert_eq!(run["key_id"], Value::Null);
    assert!(run["started_at"].as_u64().is_some(), "{run}");
    let rules_fingerprint = run["rules_fingerprint"].as_str().expect("a fingerprint");
    assert!(is_hex(rules_fingerprint, 64), "{rules_fingerprint}");
}

#[test]
fn the_lines_hold_no_message_and_no_text_of_the_code() {
    let dir = project("no-text", None);

    lint(&dir, &["--record"]);

    let all = fs::read_to_string(history(&dir).join("findings.jsonl")).expect("the findings")
        + &fs::read_to_string(history(&dir).join("runs.jsonl")).expect("the runs");
    assert!(!all.contains("Unused suppression"), "{all}");
    assert!(!all.contains("botica: ignore"), "{all}");
}

#[test]
fn a_run_without_record_writes_nothing() {
    let dir = project("off", None);

    lint(&dir, &[]);

    assert!(!history(&dir).exists());
}

#[test]
fn the_history_table_turns_recording_on_and_no_record_turns_it_off_for_one_run() {
    let dir = project("table", Some("[history]\nrecord = true\n"));

    lint(&dir, &["--no-record"]);
    assert!(!history(&dir).exists());

    lint(&dir, &[]);
    assert_eq!(runs(&dir).len(), 1);
}

#[test]
fn the_history_table_leaves_the_rules_fingerprint_as_it_is() {
    let off = project("fingerprint-off", None);
    let on = project("fingerprint-on", Some("[history]\nrecord = true\n"));

    lint(&off, &["--record"]);
    lint(&on, &[]);

    assert_eq!(
        runs(&off)[0]["rules_fingerprint"],
        runs(&on)[0]["rules_fingerprint"]
    );
}

#[test]
fn the_history_ignores_itself_in_git() {
    let dir = project("ignored", None);

    lint(&dir, &["--record"]);

    let ignore = fs::read_to_string(history(&dir).join(".gitignore")).expect("the .gitignore");
    assert_eq!(ignore, "*\n");
}

#[test]
fn a_history_without_its_gitignore_gets_one() {
    let dir = project("gitignore-added", None);
    fs::create_dir_all(history(&dir)).expect("create the history");

    lint(&dir, &["--record"]);

    assert!(history(&dir).join(".gitignore").is_file());
}

#[test]
fn each_run_appends_and_a_file_without_a_final_newline_gets_one_first() {
    let dir = project("appends", None);
    lint(&dir, &["--record"]);
    let path = history(&dir).join("runs.jsonl");
    let mut text = fs::read_to_string(&path).expect("the runs");
    text.pop();
    fs::write(&path, &text).expect("drop the final newline");

    lint(&dir, &["--record"]);

    let runs = runs(&dir);
    assert_eq!(runs.len(), 2, "{runs:?}");
    assert_ne!(runs[0]["run_id"], runs[1]["run_id"]);
    assert_eq!(findings(&dir).len(), 2);
}

#[test]
fn a_run_records_whatever_its_exit_code() {
    let dir = project(
        "fails",
        Some("[rules.unused-suppression]\nseverity = \"error\"\n"),
    );

    let output = lint(&dir, &["--record"]);

    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert_eq!(runs(&dir).len(), 1);
}

#[test]
fn a_run_that_stops_on_an_error_of_the_configuration_writes_nothing() {
    let dir = project("bad-config", Some("select = [\"no-such-rule\"]\n"));

    let output = lint(&dir, &["--record"]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(!history(&dir).exists());
}

#[test]
fn a_history_that_cannot_be_written_is_a_warning_and_leaves_the_exit_code() {
    let dir = project("blocked", None);
    fs::write(dir.join(".botica"), "a file where the directory goes").expect("block it");

    let output = lint(&dir, &["--record"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("warning: the run was not recorded in the history"),
        "{}",
        stderr(&output)
    );
    assert!(stdout(&output).contains("unused-suppression"));
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_in_place_of_the_history_or_one_of_its_files_is_refused() {
    let dir = project("linked", None);
    let elsewhere = dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("create the target");
    fs::create_dir_all(dir.join(".botica")).expect("create .botica");
    std::os::unix::fs::symlink(&elsewhere, history(&dir)).expect("link the history");

    let output = lint(&dir, &["--record"]);

    assert!(
        stderr(&output).contains("symbolic link"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fs::read_dir(&elsewhere).expect("the target").count(), 0);

    fs::remove_file(history(&dir)).expect("remove the link");
    fs::create_dir_all(history(&dir)).expect("create the history");
    let target = elsewhere.join("runs.jsonl");
    std::os::unix::fs::symlink(&target, history(&dir).join("runs.jsonl")).expect("link a file");

    let output = lint(&dir, &["--record"]);

    assert!(
        stderr(&output).contains("runs.jsonl"),
        "{}",
        stderr(&output)
    );
    assert!(!target.exists());
}

#[test]
fn the_key_names_itself_in_key_id_and_is_written_nowhere() {
    let dir = project("keyed", None);

    lint_with(&dir, &[("BOTICA_HISTORY_KEY", KEY)], &["--record"]);
    lint(&dir, &["--record"]);

    let runs = runs(&dir);
    let key_id = runs[0]["key_id"].as_str().expect("a key id");
    assert!(is_hex(key_id, 8), "{key_id}");
    assert_eq!(runs[1]["key_id"], Value::Null);
    let found = findings(&dir);
    assert_ne!(found[0]["fingerprint"], found[1]["fingerprint"]);
    for file in ["runs.jsonl", "findings.jsonl"] {
        let text = fs::read_to_string(history(&dir).join(file)).expect("the file");
        assert!(!text.contains(KEY), "{text}");
    }
}

#[test]
fn a_key_too_short_records_nothing_with_a_warning() {
    let dir = project("short-key", None);

    let output = lint_with(&dir, &[("BOTICA_HISTORY_KEY", "short")], &["--record"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("needs at least 32"),
        "{}",
        stderr(&output)
    );
    assert!(!stderr(&output).contains("short\""), "{}", stderr(&output));
    assert!(runs(&dir).is_empty());
}

#[test]
fn require_key_records_nothing_without_the_key() {
    let dir = project(
        "require-key",
        Some("[history]\nrecord = true\nrequire-key = true\n"),
    );

    let output = lint(&dir, &[]);
    assert!(
        stderr(&output).contains("require-key"),
        "{}",
        stderr(&output)
    );
    assert!(runs(&dir).is_empty());

    lint_with(&dir, &[("BOTICA_HISTORY_KEY", KEY)], &[]);
    assert_eq!(runs(&dir).len(), 1);
}

#[test]
fn equal_findings_of_one_run_are_told_apart_by_their_occurrence() {
    let dir = project_in("lint-record", "occurrence", None);
    write(
        &dir,
        "a.py",
        "x = 1  # botica: ignore[no-utils]\nx = 1  # botica: ignore[no-utils]\n\
         x = 1  # botica: ignore[no-utils]\nx = 1  # botica: ignore[no-utils]\n",
    );

    lint(&dir, &["--record"]);

    let found = findings(&dir);
    assert_eq!(found.len(), 4, "{found:?}");
    assert_eq!(found[1]["fingerprint"], found[2]["fingerprint"]);
    assert_eq!(found[1]["occurrence"], 0);
    assert_eq!(found[2]["occurrence"], 1);
}
