mod common;

use common::{botica, command, lines, project_in, stderr, stdout, write};
use serde_json::json;
use std::path::PathBuf;

/// Two `no-utils` findings and one `file-size-guard`.
fn project(name: &str) -> PathBuf {
    let dir = project_in("statistics", name, None);
    write(&dir, "utils.py", "x = 1\n");
    write(&dir, "pkg/utils.py", "x = 1\n");
    write(&dir, "big.py", &lines(701));
    dir
}

#[test]
fn text_is_a_count_per_rule_the_most_frequent_first_instead_of_the_findings() {
    let dir = project("text");
    let output = botica(&dir, &[".", "--statistics"]);

    let rows: Vec<Vec<String>> = stdout(&output)
        .lines()
        .map(|line| line.split_whitespace().map(str::to_owned).collect())
        .collect();
    assert_eq!(
        rows,
        [
            ["2", "BTSTRUC002", "no-utils"],
            ["1", "BTSTRUC001", "file-size-guard"]
        ]
    );
    assert_eq!(output.status.code(), Some(1), "the exit does not change");
}

#[test]
fn rules_with_the_same_count_come_in_name_order() {
    let dir = project_in("statistics", "ties", None);
    write(&dir, "utils.py", "x = 1\n");
    write(&dir, "big.py", &lines(701));
    let text = stdout(&botica(&dir, &[".", "--statistics"]));
    let slugs: Vec<&str> = text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .collect();
    assert_eq!(slugs, ["file-size-guard", "no-utils"]);
}

#[test]
fn json_is_a_document_with_the_counts_and_the_errors_of_the_run() {
    let dir = project("json");
    let output = botica(&dir, &[".", "--statistics", "--output-format", "json"]);

    let document: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("one JSON document");
    assert_eq!(
        document,
        json!({
            "schema_version": 1,
            "statistics": [
                {"rule": "no-utils", "code": "BTSTRUC002", "count": 2},
                {"rule": "file-size-guard", "code": "BTSTRUC001", "count": 1}
            ],
            "errors": []
        })
    );
}

#[test]
fn a_rule_the_catalog_does_not_know_has_no_code() {
    let dir = project_in("statistics", "unknown", None);
    write(&dir, "log.py", "print('x')\n");
    let text = stdout(&command(&dir, &["lint", ".", "--statistics"]));
    assert_eq!(
        text.split_whitespace().collect::<Vec<_>>(),
        ["1", "-", "log-no-print"]
    );
    let json = stdout(&command(
        &dir,
        &["lint", ".", "--statistics", "--output-format", "json"],
    ));
    let document: serde_json::Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(
        document["statistics"],
        json!([{"rule": "log-no-print", "code": null, "count": 1}])
    );
}

#[test]
fn a_clean_run_has_no_rows() {
    let dir = project_in("statistics", "clean", None);
    write(&dir, "a.py", "x = 1\n");
    let output = botica(&dir, &[".", "--statistics"]);
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(0));
    let json = botica(&dir, &[".", "--statistics", "--output-format", "json"]);
    assert!(
        stdout(&json).contains("\"statistics\":[]"),
        "{}",
        stdout(&json)
    );
}

#[test]
fn exit_zero_still_forgives_the_findings() {
    let dir = project("exit-zero");
    let output = botica(&dir, &[".", "--statistics", "--exit-zero"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn only_text_and_json_can_be_counted() {
    let dir = project("formats");
    for format in ["sarif", "github"] {
        let output = botica(&dir, &[".", "--statistics", "--output-format", format]);
        assert_eq!(output.status.code(), Some(2), "{format}");
        assert!(stdout(&output).is_empty(), "{format}");
        assert!(
            stderr(&output).contains("--statistics") && stderr(&output).contains(format),
            "{}",
            stderr(&output)
        );
    }
}
