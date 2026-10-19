//! `botica-lsp`: the columns of the protocol, in UTF-16 code units or in bytes, and the changes
//! that the editor sends as ranges to replace.

mod common;

use std::sync::Mutex;

use botica_core::Diagnostic;
use botica_lsp::{Options, Plain};
use common::{exit, finding, serve_linter, A_PY};
use serde_json::{json, Value};

fn initialize_offering(encodings: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "capabilities": {"general": {"positionEncodings": encodings}}}})
}

fn open(text: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {
        "uri": A_PY, "languageId": "python", "version": 1, "text": text}}})
}

fn edit(changes: &Value) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": A_PY, "version": 2}, "contentChanges": changes}})
}

fn range(start: (u32, u32), end: (u32, u32), text: &str) -> Value {
    json!({"range": {"start": {"line": start.0, "character": start.1},
                     "end": {"line": end.0, "character": end.1}},
           "text": text})
}

/// The texts the server lints after `messages`, the editor offering `encodings`.
fn texts_after(encodings: &Value, messages: &[Value]) -> Vec<String> {
    let texts = Mutex::new(Vec::new());
    let lint = |_: &str, text: &str| {
        texts.lock().expect("the texts").push(text.to_owned());
        Vec::new()
    };
    serve_linter(&Plain(&lint), Options::default(), |editor| {
        editor.send(&initialize_offering(encodings));
        for message in messages {
            editor.send(message);
        }
        editor.wait_for(1 + messages.len());
        editor.send(&exit());
    });
    texts.into_inner().expect("the texts")
}

/// The range of the one finding published for `text`, the editor offering `encodings`.
fn range_of(encodings: &Value, text: &str, found: Diagnostic) -> Value {
    let written = serve_linter(
        &Plain(&move |_, _| vec![found.clone()]),
        Options::default(),
        |editor| {
            editor.send(&initialize_offering(encodings));
            editor.send(&open(text));
            editor.wait_for(2);
            editor.send(&exit());
        },
    );
    written[1]["params"]["diagnostics"][0]["range"].clone()
}

fn after_the_smiley() -> Diagnostic {
    let mut found = finding("no-eval", "no");
    (found.line, found.column, found.end_line, found.end_column) = (2, 3, 2, 4);
    found
}

#[test]
fn a_column_counts_utf_16_code_units_unless_the_editor_offers_utf_8() {
    let text = "x\n😀 eval\n";

    assert_eq!(
        range_of(&json!(null), text, after_the_smiley()),
        json!({"start": {"line": 1, "character": 3}, "end": {"line": 1, "character": 4}})
    );
    assert_eq!(
        range_of(&json!(["utf-8"]), text, after_the_smiley()),
        json!({"start": {"line": 1, "character": 5}, "end": {"line": 1, "character": 6}})
    );
}

#[test]
fn a_column_past_the_end_of_its_line_keeps_the_characters_past_it() {
    let mut found = finding("no-eval", "no");
    (found.column, found.end_column) = (4, 6);

    assert_eq!(
        range_of(&json!(null), "é\n", found),
        json!({"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 5}})
    );
}

#[test]
fn a_change_replaces_its_range_counted_in_the_encoding_of_the_session() {
    let utf16 = texts_after(
        &json!(null),
        &[
            open("😀a\nhé!\n"),
            edit(&json!([range((0, 2), (0, 3), "b")])),
        ],
    );
    let utf8 = texts_after(
        &json!(["utf-8"]),
        &[
            open("😀a\nhé!\n"),
            edit(&json!([range((1, 3), (1, 4), "?")])),
        ],
    );

    assert_eq!(utf16, ["😀a\nhé!\n", "😀b\nhé!\n"]);
    assert_eq!(utf8, ["😀a\nhé!\n", "😀a\nhé?\n"]);
}

#[test]
fn the_changes_of_one_message_apply_in_their_order_and_the_text_is_linted_once() {
    let texts = texts_after(
        &json!(null),
        &[
            open("one\ntwo\n"),
            edit(&json!([
                range((0, 0), (0, 3), "uno"),
                range((1, 0), (2, 0), ""),
                range((1, 0), (1, 0), "dos\n"),
            ])),
        ],
    );

    assert_eq!(texts, ["one\ntwo\n", "uno\ndos\n"]);
}

#[test]
fn a_range_past_the_end_of_a_line_or_of_the_text_stops_there() {
    let texts = texts_after(
        &json!(null),
        &[
            open("ab\ncd"),
            edit(&json!([range((0, 1), (0, 99), "X")])),
            edit(&json!([range((7, 0), (9, 9), "!")])),
        ],
    );

    assert_eq!(texts, ["ab\ncd", "aX\ncd", "aX\ncd!"]);
}

#[test]
fn a_change_without_a_range_replaces_the_whole_text() {
    let texts = texts_after(
        &json!(null),
        &[
            open("old\n"),
            edit(&json!([range((0, 0), (0, 1), "b"), {"text": "new\n"}])),
        ],
    );

    assert_eq!(texts, ["old\n", "new\n"]);
}
