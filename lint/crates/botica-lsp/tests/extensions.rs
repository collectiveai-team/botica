//! `botica-lsp` with extension rules: the editor asks for them, a rule starts with the first
//! document it reads, and what it finds is added to what the engine found.

mod common;

use std::sync::Arc;

use botica_lsp::Options;
use common::{
    asking, change, exit, finding, finds_something, initialize, messages_of, on, open, saw, serve,
    shutdown, Asked, Fake, Gate, A_PY,
};
use serde_json::{json, Value};

#[test]
fn what_an_extension_finds_is_added_to_what_the_engine_found() {
    let fake = Fake::new(|asked, _| Ok(vec![finding(&asked.key, "from the extension")]));

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "x = 1\n"));
        editor.wait_for(3);
        editor.send(&exit());
    });

    assert_eq!(messages_of(&written[1]), ["from the engine"]);
    assert_eq!(
        messages_of(&written[2]),
        ["from the engine", "from the extension"]
    );
    assert_eq!(written[2]["params"]["uri"], A_PY);
}

#[test]
fn the_extensions_do_not_run_unless_the_editor_asks_for_them() {
    for options in [json!({}), json!({"extensions": false}), Value::Null] {
        let fake = finds_something();

        let written = serve(&fake, on(&fake), |editor| {
            editor.send(&initialize(&options));
            editor.send(&open(A_PY, 1, "x = 1\n"));
            editor.wait_for(2);
            editor.send(&shutdown(2));
            editor.wait_for(3);
            editor.send(&exit());
        });

        assert_eq!(written.len(), 3, "options {options}");
        assert_eq!(messages_of(&written[1]), ["from the engine"]);
        assert!(fake.starts().is_empty(), "options {options}");
    }
}

#[test]
fn the_flag_of_the_server_asks_for_the_extensions_as_the_option_does() {
    let fake = finds_something();
    let flag = Options {
        enabled: true,
        ..on(&fake)
    };

    let written = serve(&fake, flag, |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "x = 1\n"));
        editor.wait_for(3);
        editor.send(&exit());
    });

    assert_eq!(
        messages_of(&written[2]),
        ["from the engine", "from the extension"]
    );
}

#[test]
fn a_rule_starts_when_the_first_document_it_reads_opens_and_once() {
    let fake = finds_something();

    serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&shutdown(2));
        editor.wait_for(2);
        assert_eq!(fake.starts(), Vec::<String>::new(), "before a document");

        editor.send(&open(A_PY, 1, "x\n"));
        editor.wait_for(4);
        assert_eq!(fake.starts(), ["a"], "after the first document of `a`");

        editor.send(&open("file:///work/app/other.py", 1, "y\n"));
        editor.wait_for(6);
        assert_eq!(fake.starts(), ["a"], "a second document of `a`");

        editor.send(&open("file:///work/app/c.ts", 1, "z\n"));
        editor.wait_for(8);
        assert_eq!(fake.starts(), ["a", "b"], "a document of `b`");
        editor.send(&exit());
    });
}

#[test]
fn a_slow_rule_does_not_hold_the_findings_of_the_others() {
    let gate = Arc::new(Gate::default());
    let held = Arc::clone(&gate);
    let mut fake = Fake::new(move |asked, _| {
        if asked.key == "slow" {
            held.wait();
        }
        Ok(vec![finding(&asked.key, &format!("from {}", asked.key))])
    });
    fake.keys = |_| vec!["slow".to_owned(), "fast".to_owned()];

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "x\n"));
        editor.wait_for(3);
        gate.open();
        editor.wait_for(4);
        editor.send(&exit());
    });

    assert_eq!(
        messages_of(&written[2]),
        ["from the engine", "from fast"],
        "while `slow` is held"
    );
    assert_eq!(
        messages_of(&written[3]),
        ["from the engine", "from fast", "from slow"]
    );
}

#[test]
fn the_rule_is_given_the_text_of_the_buffer_with_its_path_from_the_root() {
    let fake = Fake::new(|asked, _| Ok(saw(asked)));

    serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open("file:///work/app/src/a.py", 1, "from the buffer\n"));
        editor.wait_for(3);
        editor.send(&exit());
    });

    assert_eq!(
        fake.asked(),
        [Asked {
            key: "a".to_owned(),
            path: "src/a.py".to_owned(),
            text: "from the buffer\n".to_owned(),
        }]
    );
}

#[test]
fn a_rule_that_runs_at_save_is_not_given_the_changes_and_keeps_what_it_found() {
    let mut fake = Fake::new(|asked, _| Ok(saw(asked)));
    fake.saved_keys = |_| vec!["s".to_owned()];

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(4);
        editor.send(&change(A_PY, 2, "two\n"));
        editor.wait_for(6);
        editor.send(&json!({"jsonrpc": "2.0", "method": "textDocument/didSave",
            "params": {"textDocument": {"uri": A_PY}}}));
        editor.wait_for(8);
        editor.send(&exit());
    });

    assert_eq!(messages_of(&written[4]), ["from the engine", "saw one"]);
    assert_eq!(
        messages_of(&written[5]),
        ["from the engine", "saw two", "saw one"]
    );
    assert_eq!(
        messages_of(&written[7]),
        ["from the engine", "saw two", "saw two"]
    );
    let saved: Vec<String> = fake
        .asked()
        .into_iter()
        .filter(|asked| asked.key == "s")
        .map(|asked| asked.text)
        .collect();
    assert_eq!(saved, ["one\n", "two\n"]);
}
