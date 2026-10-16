//! `botica server` with the extension rules of the project: the flag or the option of the editor
//! asks for them, and what they find in a buffer follows what the engine found.

mod common;

use std::path::PathBuf;

use common::extension::{
    add_rule, folder, implementation, project, rule_toml, script, starts, README,
};
use common::lsp::{initialize, open, Session};
use common::write;
use serde_json::{json, Value};

/// The project with the rule `acme/by-sh`, whose process answers with a finding of the rule for
/// each file it is sent.
fn with_a_rule(name: &str, config: &str) -> PathBuf {
    let dir = project(name, config);
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    dir
}

fn uri_of(dir: &std::path::Path, file: &str) -> String {
    format!("file://{}/{file}", dir.display())
}

/// How the editor asks for the extensions: the flag of the command, or the option of `initialize`.
fn ways_to_ask() -> [(&'static [&'static str], Value); 2] {
    [
        (&["server", "--extensions"], json!({})),
        (&["server"], json!({"extensions": true})),
    ]
}

fn diagnostics(published: &Value) -> &Vec<Value> {
    published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
}

#[test]
fn what_an_extension_finds_in_a_buffer_follows_the_findings_of_the_engine() {
    for (args, options) in ways_to_ask() {
        let dir = with_a_rule("asked", "");
        let uri = uri_of(&dir, "scratch.py");

        let mut editor = Session::start(&dir, args);
        editor.send(&initialize(&options));
        editor.read();
        editor.send(&open(&uri, "x = 1\n"));
        let engine = editor.read();
        let extension = editor.read();
        let (_, status) = editor.finish();

        assert!(status.success());
        assert!(diagnostics(&engine).is_empty(), "{args:?}");
        assert_eq!(extension["params"]["uri"], uri);
        let found = &diagnostics(&extension)[0];
        assert_eq!(found["code"], "acme/by-sh", "{args:?}");
        assert_eq!(found["severity"], 1);
        assert_eq!(found["range"]["start"], json!({"line": 0, "character": 0}));
        assert_eq!(starts(&dir), [format!("acme/by-sh {}", dir.display())]);
    }
}

#[test]
fn no_extension_runs_unless_the_editor_asks_for_them() {
    let dir = with_a_rule("not-asked", "");

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&json!({"extensions": false})));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    let engine = editor.read();
    let (rest, status) = editor.finish();

    assert!(status.success());
    assert!(diagnostics(&engine).is_empty());
    assert_eq!(rest.len(), 1, "only the reply to shutdown follows");
    assert!(starts(&dir).is_empty());
}

#[test]
fn a_rule_of_project_scope_does_not_run_on_a_buffer() {
    let dir = with_a_rule("project-scope", "");
    let rule = format!("{}/rule.toml", folder("by-sh"));
    let text = std::fs::read_to_string(dir.join(&rule)).expect("the rule");
    write(
        &dir,
        &rule,
        &text.replace("scope = \"file\"", "scope = \"project\""),
    );

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    editor.read();
    let (rest, status) = editor.finish();

    assert!(status.success());
    assert_eq!(rest.len(), 1, "only the reply to shutdown follows");
    assert!(starts(&dir).is_empty());
}

#[test]
fn the_configuration_names_the_severity_of_what_an_extension_finds() {
    let dir = with_a_rule("severity", "[rules.\"acme/by-sh\"]\nseverity = \"info\"\n");

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    editor.read();
    let extension = editor.read();
    editor.finish();

    assert_eq!(diagnostics(&extension)[0]["severity"], 3);
}

#[test]
fn a_comment_of_the_buffer_silences_what_an_extension_finds() {
    let dir = with_a_rule("silenced", "");

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(
        &uri_of(&dir, "scratch.py"),
        "x = 1  # botica: ignore[acme/by-sh]\n",
    ));
    editor.read();
    let extension = editor.read();
    editor.finish();

    assert!(diagnostics(&extension).is_empty(), "{extension}");
}

#[test]
fn the_end_of_the_session_ends_the_process_of_each_rule() {
    let dir = with_a_rule("ending", "");
    let ending =
        script("acme/by-sh", "error").replace("exit 0 ;;", "echo stopped >> stops.log; exit 0 ;;");
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), &ending);

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    editor.read();
    editor.read();
    let (_, status) = editor.finish();

    assert!(status.success());
    let stops = std::fs::read_to_string(dir.join("stops.log")).unwrap_or_default();
    assert_eq!(stops.trim(), "stopped");
}

#[test]
fn a_finding_about_another_rule_is_left_out() {
    let dir = project("impostor", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/other");

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    editor.read();
    let extension = editor.read();
    editor.finish();

    assert!(diagnostics(&extension).is_empty(), "{extension}");
}

#[test]
fn the_process_starts_at_the_root_of_the_project_whatever_the_directory_of_the_server() {
    let dir = with_a_rule("from-below", "");
    let below = dir.join("below");
    std::fs::create_dir_all(&below).expect("a subdirectory");

    let mut editor = Session::start(&below, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&below, "scratch.py"), "x = 1\n"));
    editor.read();
    editor.read();
    editor.finish();

    assert_eq!(starts(&dir), [format!("acme/by-sh {}", dir.display())]);
}

#[test]
fn each_implementation_of_a_rule_is_a_process_of_its_own() {
    let dir = project("two-implementations", "");
    let root = folder("by-sh");
    let how = |script: &str| format!("command = [\"sh\", \"{root}/{script}\"]");
    let implementations = [
        implementation("command", "\"python\"", &how("first.sh")),
        implementation("command", "\"typescript\"", &how("second.sh")),
    ]
    .join("\n");
    write(
        &dir,
        &format!("{root}/rule.toml"),
        &rule_toml("by-sh", "ACMESTRUC001", "", &implementations),
    );
    write(&dir, &format!("{root}/README.md"), README);
    let first = script("acme/by-sh", "error");
    write(&dir, &format!("{root}/first.sh"), &first);
    write(
        &dir,
        &format!("{root}/second.sh"),
        &first.replace("echo \"acme/by-sh", "echo \"second"),
    );

    let mut editor = Session::start(&dir, &["server", "--extensions"]);
    editor.send(&initialize(&Value::Null));
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.py"), "x = 1\n"));
    editor.read();
    editor.read();
    editor.send(&open(&uri_of(&dir, "scratch.ts"), "let x = 1;\n"));
    editor.read();
    let typescript = editor.read();
    editor.finish();

    assert_eq!(diagnostics(&typescript)[0]["code"], "acme/by-sh");
    assert_eq!(
        starts(&dir),
        [
            format!("acme/by-sh {}", dir.display()),
            format!("second {}", dir.display())
        ]
    );
}
