//! `botica lint --mutants <outcomes.json>`: the mutants that survived a run of cargo-mutants, as
//! findings of `mutant-survived`.

mod common;

use common::record::{findings, lint, runs};
use common::{project_in, stderr, stdout, write};
use serde_json::{json, Value};
use std::path::PathBuf;

const SELECTED: &str = "preview = true\nselect = [\"mutant-survived\"]\n";

/// The source that the mutants of the report change.
const LIB: &str = "pub fn is_even(n: u32) -> bool {\n    n % 2 == 0\n}\n\n\
                   pub fn double(n: u32) -> u32 {\n    n * 2\n}\n";

/// One outcome of a mutant of `src/lib.rs` as cargo-mutants 27.1.0 writes it in
/// `mutants.out/outcomes.json` (`src/outcome.rs`, `src/mutant.rs`).
fn mutant(change: &str, start: (u32, u32), end: (u32, u32), summary: &str) -> Value {
    json!({
        "scenario": {"Mutant": {
            "name": format!("src/lib.rs:{}:{}: {change}", start.0, start.1),
            "package": "demo",
            "file": "src/lib.rs",
            "function": null,
            "span": {
                "start": {"line": start.0, "column": start.1},
                "end": {"line": end.0, "column": end.1}
            },
            "replacement": "",
            "genre": "BinaryOperator"
        }},
        "summary": summary,
        "log_path": "log/x.log",
        "diff_path": "diff/x.diff",
        "phase_results": []
    })
}

/// A report with the baseline, two mutants that survived and one of each other outcome.
fn outcomes() -> String {
    json!({
        "outcomes": [
            {"scenario": "Baseline", "summary": "Success", "log_path": "log/baseline.log",
             "diff_path": null, "phase_results": []},
            mutant("replace == with != in is_even", (2, 11), (2, 13), "MissedMutant"),
            mutant("replace double -> u32 with 0", (5, 30), (7, 2), "MissedMutant"),
            mutant("replace * with + in double", (6, 7), (6, 8), "CaughtMutant"),
            mutant("replace is_even -> bool with false", (1, 32), (3, 2), "Timeout"),
            mutant("replace double -> u32 with 1", (6, 5), (6, 10), "Unviable"),
        ],
        "total_mutants": 5, "missed": 2, "caught": 1, "timeout": 1, "unviable": 1, "success": 0,
        "start_time": "2026-11-12T10:00:00Z", "end_time": "2026-11-12T10:00:30Z",
        "cargo_mutants_version": "27.1.0"
    })
    .to_string()
}

fn project(name: &str, config: Option<&str>, lib: &str) -> PathBuf {
    let dir = project_in("lint-mutants", name, config);
    write(&dir, "src/lib.rs", lib);
    write(&dir, "outcomes.json", &outcomes());
    dir
}

#[test]
fn each_missed_mutant_is_a_warning_at_its_line_and_the_others_are_left_out() {
    let dir = project("missed", Some(SELECTED), LIB);

    let output = lint(&dir, &["--mutants", "outcomes.json"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "src/lib.rs:2:11: warning [mutant-survived] The tests pass with this mutant of \
         cargo-mutants: replace == with != in is_even. Add a test that fails on it, or delete \
         the code it changes\n\
         src/lib.rs:5:30: warning [mutant-survived] The tests pass with this mutant of \
         cargo-mutants: replace double -> u32 with 0. Add a test that fails on it, or delete the \
         code it changes\n"
    );
    let gate = lint(
        &dir,
        &["--mutants", "outcomes.json", "--fail-on", "warning"],
    );
    assert_eq!(gate.status.code(), Some(1));
}

#[test]
fn a_project_that_does_not_select_the_rule_reports_nothing() {
    let dir = project("unselected", None, LIB);

    let output = lint(
        &dir,
        &["--mutants", "outcomes.json", "--fail-on", "warning"],
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn with_a_diff_only_the_mutants_on_the_lines_it_adds_stay() {
    let dir = project("diff", Some(SELECTED), LIB);
    write(
        &dir,
        "change.diff",
        "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -5,0 +5,3 @@\n+pub fn double(n: u32) -> u32 {\n\
         +    n * 2\n+}\n",
    );

    let output = lint(
        &dir,
        &["--mutants", "outcomes.json", "--diff-file", "change.diff"],
    );

    let text = stdout(&output);
    assert!(text.contains("src/lib.rs:5:30:"), "{text}");
    assert!(!text.contains("src/lib.rs:2:11:"), "{text}");
}

#[test]
fn a_comment_with_a_reason_on_the_line_of_a_mutant_silences_it() {
    let lib = LIB.replace(
        "    n % 2 == 0\n",
        "    n % 2 == 0 // botica: ignore[mutant-survived] -- covered by the property tests\n",
    );
    let dir = project("silenced", Some(SELECTED), &lib);

    let output = lint(&dir, &["--mutants", "outcomes.json"]);

    let text = stdout(&output);
    assert!(!text.contains("src/lib.rs:2:"), "{text}");
    assert!(text.contains("src/lib.rs:5:30:"), "{text}");
}

#[test]
fn the_walk_accepts_a_comment_about_the_rule_and_asks_it_for_a_reason() {
    let with_reason = LIB.replace(
        "    n % 2 == 0\n",
        "    n % 2 == 0 // botica: ignore[mutant-survived] -- covered by the property tests\n",
    );
    let without = LIB.replace(
        "    n % 2 == 0\n",
        "    n % 2 == 0 // botica: ignore[mutant-survived]\n",
    );
    let config = "preview = true\nselect = [\"mutant-survived\", \"unused-suppression\", \
                  \"invalid-suppression\"]\n";
    let accepted = project("walk-reason", Some(config), &with_reason);
    let refused = project("walk-no-reason", Some(config), &without);

    let accepted = lint(&accepted, &[]);
    let refused = lint(&refused, &[]);

    assert_eq!(stdout(&accepted), "", "{}", stderr(&accepted));
    assert!(
        stdout(&refused).contains(
            "[invalid-suppression] A suppression of `mutant-survived` needs a reason after `--`"
        ),
        "{}",
        stdout(&refused)
    );
}

#[test]
fn a_report_that_cannot_be_read_or_parsed_fails_the_run() {
    let dir = project("broken", Some(SELECTED), LIB);
    write(&dir, "broken.json", "{\"runs\": []}");

    for (file, expected) in [
        ("missing.json", "cannot read `missing.json`"),
        (
            "broken.json",
            "the mutants are not a report of cargo-mutants (`mutants.out/outcomes.json`)",
        ),
    ] {
        let output = lint(&dir, &["--mutants", file]);

        assert_eq!(output.status.code(), Some(2), "{file}");
        assert!(
            stderr(&output).contains(expected),
            "{file}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn mutants_take_no_paths_and_no_judged_findings() {
    let dir = project("conflicts", Some(SELECTED), LIB);

    for args in [
        ["--mutants", "outcomes.json", "src"].as_slice(),
        ["--mutants", "outcomes.json", "--judged", "outcomes.json"].as_slice(),
    ] {
        assert_eq!(lint(&dir, args).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn a_recorded_run_has_the_mode_mutants_and_the_rule_with_its_hits_and_suppressions() {
    let lib = LIB.replace(
        "    n % 2 == 0\n",
        "    n % 2 == 0 // botica: ignore[mutant-survived] -- covered by the property tests\n",
    );
    let dir = project("record", Some(SELECTED), &lib);

    let output = lint(&dir, &["--mutants", "outcomes.json", "--record"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let runs = runs(&dir);
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["mode"], "mutants");
    assert_eq!(runs[0]["rules"]["mutant-survived"]["hits"], 1);
    assert_eq!(runs[0]["rules"]["mutant-survived"]["suppressed"], 1);
    let findings = findings(&dir);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0]["slug"], "mutant-survived");
    assert_eq!(findings[0]["code"], "BTTEST012");
    assert_eq!(findings[0]["path"], "src/lib.rs");
    assert_eq!(findings[0]["line"], 5);
}

#[test]
fn a_recorded_run_of_a_project_without_the_rule_evaluates_no_rule() {
    let dir = project("record-off", None, LIB);

    lint(&dir, &["--mutants", "outcomes.json", "--record"]);

    let runs = runs(&dir);
    assert_eq!(runs[0]["mode"], "mutants");
    assert_eq!(runs[0]["rules"], serde_json::json!({}));
}
