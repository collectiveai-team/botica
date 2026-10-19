//! The Rust SDK, `botica-ext-sdk`: what an extension that serves its rules answers to the requests
//! of the extension host of botica, line by line.

use std::io::Cursor;

use botica_ext_sdk::{serve_on, Finding, Rule};
use serde_json::{json, Value};

fn rules() -> Vec<Rule> {
    vec![
        Rule::new("acme/no-leading-x", &["python"], |_path, text, _options| {
            let mut offset = 0;
            let mut found = Vec::new();
            for line in text.split_inclusive('\n') {
                if line.starts_with('x') {
                    found.push(Finding::new(offset, offset + 1, "a line starts with x"));
                }
                offset += line.len();
            }
            found
        })
        .severity("error"),
        Rule::new(
            "acme/js-anything",
            &["javascript"],
            |_path, _text, _options| vec![Finding::new(0, 1, "js")],
        ),
        Rule::new(
            "acme/ts-options",
            &["typescript"],
            |_path, _text, options| vec![Finding::new(0, 1, options.to_string())],
        ),
        Rule::project("acme/project-files", &["python"], |root, paths, options| {
            let mut found: Vec<Finding> = paths
                .iter()
                .map(|path| Finding::in_file(path, 0, 1, format!("{root} {options}")))
                .collect();
            found.push(Finding::new(0, 1, "a finding with no path"));
            found
        })
        .severity("info"),
    ]
}

/// The lines that the SDK writes when it is given `requests`, one on each line, and stdin ends.
fn replies(requests: &[&str]) -> Vec<Value> {
    let input = Cursor::new(format!("{}\n", requests.join("\n")));
    let mut output = Vec::new();

    serve_on(&rules(), input, &mut output).expect("the SDK serves");

    String::from_utf8(output)
        .expect("UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("a line of JSON"))
        .collect()
}

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"client":"botica"}}"#;
const SHUTDOWN: &str = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#;

#[test]
fn initialize_lists_the_rules_with_their_languages() {
    let lines = replies(&[INITIALIZE, SHUTDOWN]);

    assert_eq!(
        lines[0],
        json!({"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": 1, "rules": [
            {"id": "acme/no-leading-x", "languages": ["python"]},
            {"id": "acme/js-anything", "languages": ["javascript"]},
            {"id": "acme/ts-options", "languages": ["typescript"]},
            {"id": "acme/project-files", "languages": ["python"]},
        ]}})
    );
}

#[test]
fn lint_files_answers_the_findings_of_the_rules_of_the_language_of_each_file() {
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"ok = 1\nx = 2\n"},{"path":"b.js","text":"x\n"}]}}"#;

    let lines = replies(&[files, SHUTDOWN]);

    assert_eq!(
        lines[0],
        json!({"jsonrpc": "2.0", "id": 2, "result": {"diagnostics": [
            {"path": "a.py", "rule": "acme/no-leading-x", "severity": "error",
             "start": 7, "end": 8, "message": "a line starts with x"},
            {"path": "b.js", "rule": "acme/js-anything", "severity": "warning",
             "start": 0, "end": 1, "message": "js"},
        ]}})
    );
}

#[test]
fn lint_files_runs_only_the_rules_that_the_request_names() {
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"x\n"},{"path":"b.js","text":"x\n"}],"rules":["acme/js-anything","acme/project-files"]}}"#;

    let lines = replies(&[files, SHUTDOWN]);

    assert_eq!(
        lines[0],
        json!({"jsonrpc": "2.0", "id": 2, "result": {"diagnostics": [
            {"path": "b.js", "rule": "acme/js-anything", "severity": "warning",
             "start": 0, "end": 1, "message": "js"},
        ]}})
    );
}

#[test]
fn lint_files_gives_each_rule_its_own_options_and_an_empty_object_when_it_has_none() {
    let with = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"c.ts","text":"x\n"}],"options":{"acme/ts-options":{"max":3},"acme/js-anything":{"max":9}}}}"#;
    let without = r#"{"jsonrpc":"2.0","id":4,"method":"lint/files","params":{"files":[{"path":"c.ts","text":"x\n"}]}}"#;

    let lines = replies(&[with, without, SHUTDOWN]);

    assert_eq!(
        lines[0]["result"]["diagnostics"][0]["message"],
        r#"{"max":3}"#
    );
    assert_eq!(lines[1]["result"]["diagnostics"][0]["message"], "{}");
}

#[test]
fn lint_project_answers_the_findings_of_the_project_rules_in_the_files_they_name() {
    let project = r#"{"jsonrpc":"2.0","id":5,"method":"lint/project","params":{"root":"/repo","files":["a.py","b/c.py"],"options":{"acme/project-files":{"depth":2}}}}"#;

    let lines = replies(&[project, SHUTDOWN]);

    assert_eq!(
        lines[0],
        json!({"jsonrpc": "2.0", "id": 5, "result": {"diagnostics": [
            {"path": "a.py", "rule": "acme/project-files", "severity": "info",
             "start": 0, "end": 1, "message": r#"/repo {"depth":2}"#},
            {"path": "b/c.py", "rule": "acme/project-files", "severity": "info",
             "start": 0, "end": 1, "message": r#"/repo {"depth":2}"#},
        ]}})
    );
}

#[test]
fn lint_project_runs_only_the_project_rules_that_the_request_names() {
    let named = r#"{"jsonrpc":"2.0","id":5,"method":"lint/project","params":{"root":"/repo","files":["a.py"],"rules":["acme/no-leading-x"]}}"#;
    let unnamed = r#"{"jsonrpc":"2.0","id":6,"method":"lint/project","params":{"root":"/repo","files":["a.py"]}}"#;

    let lines = replies(&[named, unnamed, SHUTDOWN]);

    assert_eq!(lines[0]["result"], json!({"diagnostics": []}));
    assert_eq!(
        lines[1]["result"]["diagnostics"],
        json!([{"path": "a.py", "rule": "acme/project-files", "severity": "info",
                "start": 0, "end": 1, "message": "/repo {}"}])
    );
}

#[test]
fn an_unknown_method_gets_an_error_and_the_serving_goes_on() {
    let lines = replies(&[r#"{"jsonrpc":"2.0","id":7,"method":"nope"}"#, SHUTDOWN]);

    assert_eq!(
        lines,
        [
            json!({"jsonrpc": "2.0", "id": 7, "error": {"code": -32601, "message": "nope"}}),
            json!({"jsonrpc": "2.0", "id": 3, "result": null}),
        ]
    );
}

#[test]
fn serving_ends_after_shutdown_and_leaves_the_rest_of_the_input_unread() {
    let lines = replies(&[SHUTDOWN, INITIALIZE]);

    assert_eq!(lines, [json!({"jsonrpc": "2.0", "id": 3, "result": null})]);
}
