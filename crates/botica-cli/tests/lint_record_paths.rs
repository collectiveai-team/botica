//! The paths that a run of `botica lint --record` keeps out of the history, and the flags that
//! turn it on and off (ADR 0019 decisions 1, 5 and 7).

mod common;

use common::record::{findings, lint, project, repository, runs, UNUSED};
use common::{stdout, write};
use serde_json::Value;

#[cfg(unix)]
#[test]
fn a_file_that_a_symbolic_link_takes_out_of_the_root_is_kept_out_of_the_history() {
    let dir = project("linked-out", None);
    let outside = dir.parent().expect("has a parent").join("linked-target");
    write(&outside, "secret.py", UNUSED);
    let _ = std::fs::remove_file(dir.join("sub"));
    std::os::unix::fs::symlink(&outside, dir.join("sub")).expect("link");

    let output = lint(
        &dir,
        &["--record", "--output-format", "json", "sub/secret.py"],
    );

    let found = findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["path"], "");
    assert_eq!(found[0]["line"], 0);
    let json: Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(
        json["diagnostics"][0]["fingerprint"], found[0]["fingerprint"],
        "the output and the history carry one fingerprint"
    );
}

#[test]
fn the_output_and_the_history_carry_one_fingerprint_for_a_finding_outside_the_root() {
    let dir = project("outside-json", None);
    let outside = dir
        .parent()
        .expect("has a parent")
        .join("outside-json-file");
    write(&outside, "o.py", UNUSED);

    let output = lint(
        &dir,
        &[
            "--record",
            "--output-format",
            "json",
            "../outside-json-file/o.py",
        ],
    );

    let json: Value = serde_json::from_str(&stdout(&output)).expect("json");
    assert_eq!(
        json["diagnostics"][0]["fingerprint"],
        findings(&dir)[0]["fingerprint"]
    );
}

#[test]
fn a_file_uri_of_a_judge_is_kept_out_of_the_history() {
    let dir = project(
        "file-uri",
        Some("preview = true\nselect = [\"test-through-interface\"]\n"),
    );
    write(&dir, "tests/test_a.py", "def test_a():\n    assert a._x\n");
    write(
        &dir,
        "judge.sarif",
        r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "test-through-interface", "message": {"text": "m"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "file:/etc/tests/test_a.py"}, "region": {"startLine": 2}}}]}]}]}"#,
    );

    lint(&dir, &["--record", "--judged", "judge.sarif"]);

    let text =
        std::fs::read_to_string(dir.join(".botica/history/findings.jsonl")).unwrap_or_default();
    assert!(!text.contains("/etc"), "{text}");
    assert!(!text.contains("file:"), "{text}");
}

#[test]
fn a_diff_file_run_evaluates_no_rule_on_a_file_that_it_does_not_lint() {
    let dir = project_with_only("diff-file", "notes.txt");
    write(
        &dir,
        "change.diff",
        "--- a/gone.py\n+++ b/gone.py\n@@ -1 +1 @@\n-x = 1\n+x = 2\n",
    );

    lint(&dir, &["--record", "--diff-file", "change.diff"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "diff");
    assert!(run["rules"].get("no-utils").is_none(), "{}", run["rules"]);
}

#[test]
fn the_last_of_record_and_no_record_wins() {
    let dir = project("last-wins", None);

    lint(&dir, &["--record", "--no-record"]);
    assert!(!dir.join(".botica").exists());

    lint(&dir, &["--no-record", "--record"]);
    assert_eq!(runs(&dir).len(), 1);
}

#[test]
fn a_message_run_with_no_message_evaluates_no_rule_of_scope_commit() {
    let dir = repository("no-message");
    write(
        &dir,
        "botica.toml",
        "preview = true\nselect = [\"conventional-commits\"]\n",
    );

    lint(&dir, &["--record", "--commits", "HEAD..HEAD"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "message");
    assert!(
        run["rules"].get("conventional-commits").is_none(),
        "{}",
        run["rules"]
    );
}

/// A project with no Python file, only `name`.
fn project_with_only(name: &str, file: &str) -> std::path::PathBuf {
    let dir = project(name, None);
    std::fs::remove_file(dir.join("a.py")).expect("remove a.py");
    write(&dir, file, "x\n");
    dir
}
