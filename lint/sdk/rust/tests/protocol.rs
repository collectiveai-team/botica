//! The Rust SDK, `botica-ext-sdk`: what an extension that serves its rules answers to the requests
//! of the extension host of botica, line by line.

use std::io::Cursor;

use botica_ext_sdk::{serve_on, Finding, Rule};
use serde_json::{json, Value};

fn rules() -> Vec<Rule> {
    vec![
        Rule::new("acme/no-leading-x", &["python"], |_path, text| {
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
        Rule::new("acme/js-anything", &["javascript"], |_path, _text| {
            vec![Finding::new(0, 1, "js")]
        }),
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
