//! `botica-lsp`: what the server answers to the messages of an editor, framed as the Language
//! Server Protocol frames them.

use std::cell::RefCell;
use std::io::Cursor;
use std::path::Path;

use botica_core::{Diagnostic, Severity};
use botica_lsp::serve_on;
use serde_json::{json, Value};

fn frame(message: &Value) -> Vec<u8> {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

fn unframe(mut bytes: &[u8]) -> Vec<Value> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let text = std::str::from_utf8(bytes).expect("UTF-8");
        let (head, rest) = text.split_once("\r\n\r\n").expect("a header");
        let length: usize = head
            .strip_prefix("Content-Length: ")
            .expect("a Content-Length")
            .parse()
            .expect("a number");
        messages.push(serde_json::from_str(&rest[..length]).expect("JSON"));
        bytes = &rest.as_bytes()[length..];
    }
    messages
}

fn finding(line: usize, column: usize, end_column: usize, severity: Severity) -> Diagnostic {
    Diagnostic {
        path: "a.py".to_owned(),
        rule: "no-utils".to_owned(),
        severity,
        start: 0,
        end: 1,
        line,
        column,
        end_line: line,
        end_column,
        file_level: false,
        message: "BTSTRUC002 (no-utils): no".to_owned(),
    }
}

/// What the server writes for these messages, and what it was asked to lint as `(path, text)`.
fn talk(
    messages: &[Value],
    lint: impl Fn(&str, &str) -> Vec<Diagnostic>,
) -> (Vec<Value>, Vec<(String, String)>) {
    let asked = RefCell::new(Vec::new());
    let input: Vec<u8> = messages.iter().flat_map(frame).collect();
    let mut output = Vec::new();

    serve_on(
        Cursor::new(input),
        &mut output,
        Path::new("/work/app"),
        &|path, text| {
            asked.borrow_mut().push((path.to_owned(), text.to_owned()));
            lint(path, text)
        },
    )
    .expect("the server serves");

    (unframe(&output), asked.into_inner())
}

fn initialize() -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}})
}

fn open(uri: &str, text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": uri, "languageId": "python", "version": 1, "text": text}}})
}

fn exit() -> Value {
    json!({"jsonrpc": "2.0", "method": "exit"})
}

#[test]
fn initialize_answers_with_incremental_sync_utf_16_and_the_name_of_the_server() {
    let (replies, _) = talk(&[initialize(), exit()], |_, _| Vec::new());

    assert_eq!(
        replies[0],
        json!({"jsonrpc": "2.0", "id": 1, "result": {
            "capabilities": {
                "positionEncoding": "utf-16",
                "textDocumentSync": {"openClose": true, "change": 2, "save": {"includeText": false}},
            },
            "serverInfo": {"name": "botica"},
        }})
    );
}

#[test]
fn the_server_counts_in_utf_8_when_the_editor_offers_it() {
    let offering = |encodings: Value| {
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "capabilities": {"general": {"positionEncodings": encodings}}}})
    };

    let (utf8, _) = talk(&[offering(json!(["utf-16", "utf-8"])), exit()], |_, _| {
        Vec::new()
    });
    let (utf32, _) = talk(&[offering(json!(["utf-32"])), exit()], |_, _| Vec::new());

    assert_eq!(
        utf8[0]["result"]["capabilities"]["positionEncoding"],
        "utf-8"
    );
    assert_eq!(
        utf32[0]["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
}

#[test]
fn opening_a_document_publishes_its_findings_as_zero_based_ranges() {
    let (replies, _) = talk(
        &[
            initialize(),
            open("file:///work/app/src/a.py", "x = 1\n"),
            exit(),
        ],
        |_, _| vec![finding(2, 3, 5, Severity::Warning)],
    );

    assert_eq!(
        replies[1],
        json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {
            "uri": "file:///work/app/src/a.py",
            "diagnostics": [{
                "range": {"start": {"line": 1, "character": 2}, "end": {"line": 1, "character": 4}},
                "severity": 2,
                "code": "no-utils",
                "source": "botica",
                "message": "BTSTRUC002 (no-utils): no",
            }],
        }})
    );
}

#[test]
fn severities_are_the_levels_of_the_protocol() {
    let (replies, _) = talk(&[open("file:///work/app/a.py", ""), exit()], |_, _| {
        [
            Severity::Error,
            Severity::Warning,
            Severity::Info,
            Severity::Hint,
        ]
        .map(|severity| finding(1, 1, 2, severity))
        .to_vec()
    });

    let levels: Vec<&Value> = replies[0]["params"]["diagnostics"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|found| &found["severity"])
        .collect();
    assert_eq!(levels, [1, 2, 3, 4]);
}

#[test]
fn a_document_is_linted_by_its_path_from_the_root_and_the_text_the_editor_holds() {
    let (_, asked) = talk(
        &[
            open("file:///work/app/my%20src/a.py", "first\n"),
            json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
                "textDocument": {"uri": "file:///work/app/my%20src/a.py", "version": 2},
                "contentChanges": [{"text": "second\n"}]}}),
            exit(),
        ],
        |_, _| Vec::new(),
    );

    assert_eq!(
        asked,
        [
            ("my src/a.py".to_owned(), "first\n".to_owned()),
            ("my src/a.py".to_owned(), "second\n".to_owned()),
        ]
    );
}

#[test]
fn a_document_outside_the_root_is_linted_by_its_whole_path() {
    let (_, asked) = talk(&[open("file:///elsewhere/b.py", "x\n"), exit()], |_, _| {
        Vec::new()
    });

    assert_eq!(asked[0].0, "/elsewhere/b.py");
}

#[test]
fn closing_a_document_clears_its_findings() {
    let (replies, _) = talk(
        &[
            open("file:///work/app/a.py", "x\n"),
            json!({"jsonrpc": "2.0", "method": "textDocument/didClose", "params": {
                "textDocument": {"uri": "file:///work/app/a.py"}}}),
            exit(),
        ],
        |_, _| vec![finding(1, 1, 2, Severity::Error)],
    );

    assert_eq!(
        replies[1]["params"],
        json!({"uri": "file:///work/app/a.py", "diagnostics": []})
    );
}

#[test]
fn a_finding_about_the_whole_file_sits_at_the_start() {
    let mut whole = finding(1, 1, 1, Severity::Warning);
    whole.file_level = true;

    let (replies, _) = talk(&[open("file:///work/app/a.py", "x\n"), exit()], |_, _| {
        vec![whole.clone()]
    });

    assert_eq!(
        replies[0]["params"]["diagnostics"][0]["range"],
        json!({"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}})
    );
}

#[test]
fn shutdown_is_answered_and_exit_ends_the_serving_with_the_rest_unread() {
    let (replies, asked) = talk(
        &[
            json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}),
            exit(),
            open("file:///work/app/a.py", "x\n"),
        ],
        |_, _| Vec::new(),
    );

    assert_eq!(
        replies,
        [json!({"jsonrpc": "2.0", "id": 2, "result": null})]
    );
    assert!(asked.is_empty());
}

#[test]
fn an_unknown_request_gets_an_error_and_an_unknown_notification_is_ignored() {
    let (replies, _) = talk(
        &[
            json!({"jsonrpc": "2.0", "id": 9, "method": "textDocument/hover", "params": {}}),
            json!({"jsonrpc": "2.0", "method": "$/setTrace", "params": {}}),
            exit(),
        ],
        |_, _| Vec::new(),
    );

    assert_eq!(
        replies,
        [json!({"jsonrpc": "2.0", "id": 9, "error": {
            "code": -32601, "message": "textDocument/hover"}})]
    );
}

#[test]
fn of_several_changes_the_last_holds_the_text() {
    let (_, asked) = talk(
        &[
            json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
                "textDocument": {"uri": "file:///work/app/a.py", "version": 2},
                "contentChanges": [{"text": "older\n"}, {"text": "newer\n"}]}}),
            exit(),
        ],
        |_, _| Vec::new(),
    );

    assert_eq!(asked, [("a.py".to_owned(), "newer\n".to_owned())]);
}

#[test]
fn a_message_without_content_length_is_an_error() {
    let mut output = Vec::new();

    let served = serve_on(
        Cursor::new(b"Content-Type: x\r\n\r\n{}".to_vec()),
        &mut output,
        Path::new("/work/app"),
        &|_, _| Vec::new(),
    );

    assert!(served.is_err());
}

#[test]
fn the_end_of_the_input_ends_the_serving() {
    let (replies, _) = talk(&[initialize()], |_, _| Vec::new());

    assert_eq!(replies.len(), 1);
}
