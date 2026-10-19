//! `botica-lsp` at a save: the rules that run at save, those of project scope, and the
//! configuration read again.

mod common;

use std::sync::Mutex;

use botica_core::{Buffer, Diagnostic};
use botica_lsp::{Linter, Options, ProjectRun, When};
use common::{change, exit, finding, initialize, messages_of, open, serve_linter, warnings, A_PY};
use serde_json::{json, Value};

/// A linter with three rules: `typed` runs as the text changes, `saved` at save and `cli` only on
/// the command line. Each finds the text it was given, after `name`. The rules of project scope
/// find `project`, and `botica.toml` holds the configuration.
struct Rules {
    name: &'static str,
    project: Option<Vec<Diagnostic>>,
    runs: &'static Mutex<usize>,
}

static NO_RUNS: Mutex<usize> = Mutex::new(0);

impl Rules {
    const fn named(name: &'static str) -> Self {
        Self {
            name,
            project: None,
            runs: &NO_RUNS,
        }
    }
}

impl Linter for Rules {
    fn open(&self, path: &str, text: &str) -> Buffer {
        Buffer::new(path, text)
    }

    fn lint(&self, buffer: &Buffer) -> Vec<Diagnostic> {
        ["typed", "saved", "cli"]
            .map(|rule| {
                let message = format!("{rule} {} {}", self.name, buffer.text().trim());
                finding(rule, &message)
            })
            .to_vec()
    }

    fn when(&self, rule: &str) -> Option<When> {
        match rule {
            "typed" => Some(When::Type),
            "saved" => Some(When::Save),
            _ => None,
        }
    }

    fn project(&self) -> Option<ProjectRun> {
        let found = self.project.clone()?;
        let runs = self.runs;
        Some(Box::new(move || {
            *runs.lock().expect("the runs") += 1;
            found
        }))
    }

    fn configures(&self, path: &str) -> bool {
        path == "botica.toml"
    }
}

fn save(uri: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {
        "textDocument": {"uri": uri}}})
}

#[test]
fn a_change_shows_the_rules_that_run_as_it_is_typed_and_keeps_those_of_the_last_save() {
    let written = serve_linter(&Rules::named("v1"), Options::default(), |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.send(&change(A_PY, 2, "two\n"));
        editor.send(&save(A_PY));
        editor.wait_for(4);
        editor.send(&exit());
    });

    assert_eq!(messages_of(&written[1]), ["typed v1 one", "saved v1 one"]);
    assert_eq!(messages_of(&written[2]), ["typed v1 two", "saved v1 one"]);
    assert_eq!(messages_of(&written[3]), ["typed v1 two", "saved v1 two"]);
}

#[test]
fn a_save_runs_the_rules_of_project_scope_and_shows_them_where_they_are() {
    static RUNS: Mutex<usize> = Mutex::new(0);
    let mut elsewhere = finding("schema", "a key is missing");
    elsewhere.path = "conf/.env.example".to_owned();
    let mut here = finding("schema", "here too");
    here.path = "a.py".to_owned();
    let rules = Rules {
        project: Some(vec![elsewhere, here]),
        runs: &RUNS,
        ..Rules::named("v1")
    };

    let written = serve_linter(&rules, Options::default(), |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.wait_for(2);
        editor.send(&save(A_PY));
        editor.wait_for(5);
        editor.send(&exit());
    });

    let project: Vec<&Value> = written[3..].iter().collect();
    let of = |uri: &str| {
        project
            .iter()
            .find(|message| message["params"]["uri"] == uri)
            .map(|message| messages_of(message))
            .expect("published")
    };
    assert_eq!(
        of("file:///work/app/conf/.env.example"),
        ["a key is missing"]
    );
    assert_eq!(of(A_PY), ["typed v1 one", "saved v1 one", "here too"]);
    assert_eq!(*RUNS.lock().expect("the runs"), 1);
}

#[test]
fn a_save_of_the_configuration_lints_the_open_documents_with_the_rules_read_again() {
    let reload = || Ok(Box::new(Rules::named("v2")) as Box<dyn Linter>);
    let options = Options {
        reload: Some(&reload),
        ..Options::default()
    };

    let written = serve_linter(&Rules::named("v1"), options, |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.send(&save("file:///work/app/botica.toml"));
        editor.wait_for(3);
        editor.send(&exit());
    });

    assert_eq!(written[2]["params"]["uri"], A_PY);
    assert_eq!(messages_of(&written[2]), ["typed v2 one", "saved v2 one"]);
}

#[test]
fn a_change_of_the_configuration_on_disk_reads_it_again() {
    let reload = || Ok(Box::new(Rules::named("v2")) as Box<dyn Linter>);
    let options = Options {
        reload: Some(&reload),
        ..Options::default()
    };

    let written = serve_linter(&Rules::named("v1"), options, |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.send(
            &json!({"jsonrpc": "2.0", "method": "workspace/didChangeWatchedFiles",
            "params": {"changes": [{"uri": "file:///work/app/botica.toml", "type": 2}]}}),
        );
        editor.wait_for(3);
        editor.send(&exit());
    });

    assert_eq!(messages_of(&written[2]), ["typed v2 one", "saved v2 one"]);
}

#[test]
fn a_configuration_that_does_not_load_leaves_the_rules_as_they_were_and_says_why() {
    let reload = || Err::<Box<dyn Linter>, _>("`rules.x` is not a rule".to_owned());
    let options = Options {
        reload: Some(&reload),
        ..Options::default()
    };

    let written = serve_linter(&Rules::named("v1"), options, |editor| {
        editor.send(&initialize(&Value::Null));
        editor.send(&open(A_PY, 1, "one\n"));
        editor.send(&save("file:///work/app/botica.toml"));
        editor.send(&change(A_PY, 2, "two\n"));
        editor.wait_for(4);
        editor.send(&exit());
    });

    assert_eq!(
        warnings(&written),
        [
            "botica: the configuration was not read again (`rules.x` is not a rule); the rules \
          stay as they were"
        ]
    );
    assert_eq!(messages_of(&written[3]), ["typed v1 two", "saved v1 one"]);
}
