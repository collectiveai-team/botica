//! `botica server`: an editor opens a buffer that is not on disk and gets the findings of the
//! active rules, as it types and when it saves.

mod common;

use common::external_rule::{complexity_rule, SELECTS_COMPLEXITY, WITHOUT_THE_HOOK};
use common::lsp::{initialize, open, Session};
use common::{project_in, write};
use serde_json::{json, Value};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

const ALWAYS: &str = "[\"cli\", \"save\", \"type\"]";

/// A project with the rule `acme/no-eval`, which runs on `run_on`.
fn acme_project(name: &str, config: &str, run_on: &str) -> std::path::PathBuf {
    let dir = project_in("server", name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    let root = ".botica/rules/acme";
    write(&dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        &dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(
        &dir,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
             status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = {run_on}\n\
             default-enabled = true\ndefault-severity = \"error\"\n\n[[implementations]]\n\
             kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n"
        ),
    );
    write(&dir, &format!("{rule}/README.md"), README);
    write(
        &dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: error\n\
         message: \"ACMESTRUC001 (acme/no-eval): do not call eval\"\n\
         rule:\n  pattern: eval($$$ARGS)\n",
    );
    dir
}

fn uri(dir: &std::path::Path, path: &str) -> String {
    format!("file://{}/{path}", dir.display())
}

fn codes(published: &Value) -> Vec<String> {
    published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|found| found["code"].as_str().expect("a code").to_owned())
        .collect()
}

fn save(uri: &str) -> Value {
    json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {
        "textDocument": {"uri": uri}}})
}

fn insert(uri: &str, line: u32, character: u32, text: &str) -> Value {
    let at = json!({"line": line, "character": character});
    json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"range": {"start": at, "end": at}, "text": text}]}})
}

#[test]
fn a_buffer_that_is_not_on_disk_gets_the_findings_of_the_rules_of_the_project() {
    let dir = acme_project("buffer", "", ALWAYS);
    let uri = uri(&dir, "scratch.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    let initialized = editor.read();
    editor.send(&open(&uri, "x = 1\neval(\"1\")\n"));
    let published = editor.read();
    let (rest, status) = editor.finish();

    assert_eq!(initialized["result"]["serverInfo"]["name"], "botica");
    assert_eq!(
        initialized["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
    assert_eq!(published["method"], "textDocument/publishDiagnostics");
    assert_eq!(published["params"]["uri"], uri);
    let found = &published["params"]["diagnostics"][0];
    assert_eq!(found["code"], "acme/no-eval");
    assert_eq!(found["severity"], 1);
    assert_eq!(found["range"]["start"], json!({"line": 1, "character": 0}));
    assert_eq!(codes(&published), ["acme/no-eval"]);
    assert_eq!(rest[0]["result"], Value::Null);
    assert!(status.success());
}

#[test]
fn a_rule_that_runs_on_the_command_line_alone_is_not_shown_in_the_editor() {
    let dir = acme_project("cli-alone", "", "[\"cli\"]");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri(&dir, "a.py"), "eval(\"1\")\n"));
    let published = editor.read();
    editor.finish();

    assert_eq!(codes(&published), Vec::<String>::new());
}

#[test]
fn a_rule_that_runs_at_save_shows_at_the_open_and_at_the_save_and_not_as_the_text_changes() {
    let dir = acme_project("at-save", "", "[\"cli\", \"save\"]");
    let a_py = uri(&dir, "a.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&a_py, "x = 1\n"));
    let opened = editor.read();
    editor.send(&insert(&a_py, 1, 0, "eval(\"1\")\n"));
    let changed = editor.read();
    editor.send(&save(&a_py));
    let saved = editor.read();
    editor.finish();

    assert_eq!(codes(&opened), Vec::<String>::new());
    assert_eq!(codes(&changed), Vec::<String>::new());
    assert_eq!(codes(&saved), ["acme/no-eval"]);
}

#[test]
fn an_edit_of_a_range_is_linted_through_the_kept_syntax_tree_in_utf_16_columns() {
    let dir = acme_project("incremental", "", ALWAYS);
    let a_py = uri(&dir, "a.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&a_py, "s = '😀'; x\n"));
    editor.read();
    editor.send(
        &json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": a_py, "version": 2},
        "contentChanges": [{"range": {"start": {"line": 0, "character": 10},
                                      "end": {"line": 0, "character": 11}},
                            "text": "eval(1)"}]}}),
    );
    let changed = editor.read();
    editor.finish();

    assert_eq!(codes(&changed), ["acme/no-eval"]);
    assert_eq!(
        changed["params"]["diagnostics"][0]["range"],
        json!({"start": {"line": 0, "character": 10}, "end": {"line": 0, "character": 17}})
    );
}

#[test]
fn a_save_of_the_configuration_lints_the_open_buffers_with_it() {
    let dir = acme_project("reload", "", ALWAYS);
    let a_py = uri(&dir, "a.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&a_py, "eval(\"1\")\n"));
    let before = editor.read();
    write(&dir, "botica.toml", "ignore = [\"acme/no-eval\"]\n");
    editor.send(&save(&uri(&dir, "botica.toml")));
    let after = editor.read();
    editor.finish();

    assert_eq!(codes(&before), ["acme/no-eval"]);
    assert_eq!(after["params"]["uri"], a_py);
    assert_eq!(codes(&after), Vec::<String>::new());
}

#[test]
fn a_configuration_that_does_not_load_is_reported_and_the_rules_stay() {
    let dir = acme_project("bad-reload", "", ALWAYS);
    let a_py = uri(&dir, "a.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&a_py, "x = 1\n"));
    editor.read();
    write(&dir, "botica.toml", "select = \n");
    editor.send(&save(&uri(&dir, "botica.toml")));
    let warned = editor.read();
    editor.send(&insert(&a_py, 1, 0, "eval(\"1\")\n"));
    let changed = editor.read();
    editor.finish();

    assert_eq!(warned["method"], "window/showMessage");
    let message = warned["params"]["message"].as_str().expect("a message");
    assert!(
        message.starts_with("botica: the configuration was not read again"),
        "{message}"
    );
    assert_eq!(codes(&changed), ["acme/no-eval"]);
}

#[test]
fn a_save_runs_the_rules_of_project_scope_and_publishes_for_the_file_they_name() {
    let dir = acme_project("project-scope", SELECTS_COMPLEXITY, ALWAYS);
    complexity_rule(&dir);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    let a_py = uri(&dir, "a.py");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&a_py, "x = 1\n"));
    editor.read();
    editor.send(&save(&a_py));
    let saved = editor.read();
    let project = editor.read();
    editor.finish();

    assert_eq!(saved["params"]["uri"], a_py);
    assert_eq!(project["params"]["uri"], uri(&dir, "prek.toml"));
    assert_eq!(codes(&project), ["external-config"]);
}
