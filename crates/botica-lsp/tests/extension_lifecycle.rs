//! `botica-lsp` with extension rules: a text is linted when it has settled, an old result is
//! dropped, a process that fails is started again up to a limit, and the end of the session ends
//! the processes.

mod common;

use std::sync::Arc;
use std::time::Duration;

use botica_lsp::Options;
use common::{
    asking, change, exit, finds_something, initialize, messages_of, on, open, saw, serve, shutdown,
    warnings, Fake, Gate, A_PY,
};
use serde_json::json;

#[test]
fn changes_that_follow_each_other_are_linted_once_with_the_last_text() {
    let fake = Fake::new(|asked, _| Ok(saw(asked)));
    let slowly = Options {
        debounce: Duration::from_millis(400),
        ..on(&fake)
    };

    let written = serve(&fake, slowly, |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "v1\n"));
        editor.wait_for(3);
        editor.send(&change(A_PY, 2, "v2\n"));
        editor.wait_for(4);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(
            fake.asked().len(),
            1,
            "the rule waits for the text to settle"
        );
        for (version, text) in [(3, "v3\n"), (4, "v4\n")] {
            editor.send(&change(A_PY, version, text));
        }
        editor.wait_for(7);
        editor.send(&shutdown(2));
        editor.wait_for(8);
        editor.send(&exit());
    });

    let texts: Vec<String> = fake.asked().into_iter().map(|asked| asked.text).collect();
    assert_eq!(texts, ["v1\n", "v4\n"]);
    assert_eq!(messages_of(&written[6]), ["from the engine", "saw v4"]);
}

#[test]
fn a_result_for_a_text_that_has_changed_is_dropped() {
    let gate = Arc::new(Gate::default());
    let held = Arc::clone(&gate);
    let fake = Fake::new(move |asked, nth| {
        if nth == 1 {
            held.wait();
        }
        Ok(saw(asked))
    });

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2);
        fake.wait_asked(1);
        editor.send(&change(A_PY, 2, "two\n"));
        editor.wait_for(3);
        gate.open();
        editor.wait_for(4);
        editor.send(&shutdown(2));
        editor.wait_for(5);
        editor.send(&exit());
    });

    assert_eq!(messages_of(&written[3]), ["from the engine", "saw two"]);
    assert!(
        written
            .iter()
            .all(|message| !message.to_string().contains("saw one")),
        "nothing says what was seen in `one`"
    );
}

#[test]
fn the_result_for_a_document_that_was_closed_is_dropped() {
    let gate = Arc::new(Gate::default());
    let held = Arc::clone(&gate);
    let fake = Fake::new(move |asked, _| {
        held.wait();
        Ok(saw(asked))
    });

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2);
        editor.send(&json!({"jsonrpc": "2.0", "method": "textDocument/didClose",
                            "params": {"textDocument": {"uri": A_PY}}}));
        editor.wait_for(3);
        gate.open();
        std::thread::sleep(Duration::from_millis(150));
        editor.send(&shutdown(2));
        editor.wait_for(4);
        editor.send(&exit());
    });

    assert_eq!(
        written.len(),
        4,
        "the reply to initialize, 2 publications, the reply to shutdown"
    );
    assert_eq!(messages_of(&written[2]), Vec::<String>::new());
}

#[test]
fn a_process_that_fails_is_started_again_and_the_editor_is_warned() {
    let fake = Fake::new(|asked, nth| {
        if nth == 1 {
            Err("it crashed".to_owned())
        } else {
            Ok(saw(asked))
        }
    });

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(4);
        editor.send(&exit());
    });

    let warned = warnings(&written);
    assert_eq!(warned.len(), 1);
    assert!(
        warned[0].contains("`a`") && warned[0].contains("it crashed"),
        "{}",
        warned[0]
    );
    assert!(warned[0].contains("starting it again"), "{}", warned[0]);
    assert_eq!(messages_of(&written[3]), ["from the engine", "saw one"]);
    assert_eq!(fake.starts(), ["a", "a"]);
}

#[test]
fn a_rule_that_keeps_failing_is_given_up_after_the_limit() {
    let fake = Fake::new(|_, _| Err("it crashed".to_owned()));
    let attempts = botica_lsp::MAX_RESTARTS + 1;

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2 + attempts);
        editor.send(&change(A_PY, 2, "two\n"));
        editor.wait_for(3 + attempts);
        std::thread::sleep(Duration::from_millis(200));
        editor.send(&shutdown(2));
        editor.wait_for(4 + attempts);
        editor.send(&exit());
    });

    let warned = warnings(&written);
    assert_eq!(warned.len(), attempts);
    for restarting in &warned[..attempts - 1] {
        assert!(restarting.contains("starting it again"), "{restarting}");
    }
    assert!(
        warned[attempts - 1].contains("will not run again"),
        "{}",
        warned[attempts - 1]
    );
    assert_eq!(fake.starts().len(), attempts, "no start after the limit");
    assert_eq!(written.len(), 4 + attempts);
}

#[test]
fn a_process_that_does_not_start_is_warned_about_as_one_that_fails() {
    let mut fake = Fake::new(|_, _| Ok(Vec::new()));
    fake.start_error = Some("no such command");
    let attempts = botica_lsp::MAX_RESTARTS + 1;

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2 + attempts);
        editor.send(&exit());
    });

    let warned = warnings(&written);
    assert_eq!(warned.len(), attempts);
    assert!(warned[0].contains("no such command"), "{}", warned[0]);
    assert!(fake.asked().is_empty());
}

#[test]
fn the_end_of_the_session_ends_every_process() {
    for by_exit in [true, false] {
        let fake = finds_something();

        serve(&fake, on(&fake), |editor| {
            editor.send(&initialize(&asking()));
            editor.send(&open(A_PY, 1, "x\n"));
            editor.send(&open("file:///work/app/c.ts", 1, "y\n"));
            editor.wait_for(5);
            if by_exit {
                editor.send(&exit());
            } else {
                editor.end();
            }
        });

        assert_eq!(fake.finished(), ["a", "b"], "ended by exit: {by_exit}");
    }
}

#[test]
fn failures_that_do_not_follow_each_other_do_not_reach_the_limit() {
    let fake = Fake::new(|asked, nth| {
        if nth % 2 == 1 {
            Err("it crashed".to_owned())
        } else {
            Ok(saw(asked))
        }
    });
    let jobs = botica_lsp::MAX_RESTARTS + 2;

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "text 1\n"));
        editor.wait_for(1 + 3);
        for version in 2..=jobs {
            let step = u32::try_from(version).expect("a small number");
            editor.send(&change(A_PY, step, &format!("text {version}\n")));
            editor.wait_for(1 + 3 * version);
        }
        editor.send(&exit());
    });

    let last = &written[written.len() - 1];
    assert_eq!(
        messages_of(last),
        ["from the engine", format!("saw text {jobs}").as_str()]
    );
    assert_eq!(warnings(&written).len(), jobs, "one failure for each text");
}

#[test]
fn a_rule_that_is_busy_skips_the_texts_that_were_replaced_meanwhile() {
    let gate = Arc::new(Gate::default());
    let held = Arc::clone(&gate);
    let fake = Fake::new(move |asked, nth| {
        if nth == 1 {
            held.wait();
        }
        Ok(saw(asked))
    });
    let after_the_debounce = Duration::from_millis(200);

    let written = serve(&fake, on(&fake), |editor| {
        editor.send(&initialize(&asking()));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2);
        fake.wait_asked(1);
        editor.send(&change(A_PY, 2, "two\n"));
        editor.wait_for(3);
        std::thread::sleep(after_the_debounce);
        editor.send(&change(A_PY, 3, "three\n"));
        editor.wait_for(4);
        std::thread::sleep(after_the_debounce);
        gate.open();
        editor.wait_for(5);
        editor.send(&exit());
    });

    let texts: Vec<String> = fake.asked().into_iter().map(|asked| asked.text).collect();
    assert_eq!(texts, ["one\n", "three\n"]);
    assert_eq!(messages_of(&written[4]), ["from the engine", "saw three"]);
}
