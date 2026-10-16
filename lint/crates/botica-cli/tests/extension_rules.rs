//! `botica lint` runs the extension rules of the catalogs of a project: one process for each rule,
//! with the files where the rule applies.

mod common;

use common::extension::{
    add_rule, add_rule_with, findings, folder, implementation, lint, project, rule_toml, starts,
};
use common::{command, stderr, stdout, write};

#[test]
fn an_extension_rule_of_a_project_runs_and_its_finding_is_reported() {
    let dir = project("runs", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "app.py:1:1: error [acme/by-sh] seen\n",
        "{}",
        stderr(&output)
    );
}

#[test]
fn each_rule_is_a_process_of_its_own_that_starts_at_the_root_of_the_project() {
    let dir = project("processes", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_rule(&dir, "other", "ACMESTRUC002", "", "acme/other");
    write(&dir, "sub/inner.py", "y = 2\n");

    let output = command(&dir.join("sub"), &["lint", "."]);

    let root = dir.display();
    assert_eq!(
        starts(&dir),
        [format!("acme/by-sh {root}"), format!("acme/other {root}")],
        "{}",
        stderr(&output)
    );
    assert_eq!(
        findings(&output),
        ["inner.py acme/by-sh", "inner.py acme/other"]
    );
}

#[test]
fn a_rule_gets_the_files_in_its_languages_and_not_the_ones_its_ignores_leave_out() {
    let dir = project("files", "");
    add_rule(
        &dir,
        "by-sh",
        "ACMESTRUC001",
        "ignores = [\"skip/**\"]",
        "acme/by-sh",
    );
    write(&dir, "skip/a.py", "x\n");
    write(&dir, "app.ts", "x\n");
    write(&dir, "notes.txt", "x\n");

    let output = lint(&dir, &[]);

    assert_eq!(
        findings(&output),
        ["app.py acme/by-sh"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_rule_is_sent_only_the_files_its_overrides_keep() {
    let dir = project(
        "overrides",
        "[[overrides]]\nfiles = [\"tests/**\"]\nignore = [\"acme/by-sh\"]\n",
    );
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, "tests/t.py", "x\n");

    let output = lint(&dir, &[]);

    assert_eq!(
        findings(&output),
        ["app.py acme/by-sh"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_rule_that_is_not_active_does_not_start() {
    let dir = project("inactive", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    for args in [&["--ignore", "acme/by-sh"][..], &["--select", "no-utils"]] {
        let output = lint(&dir, args);

        assert!(
            findings(&output).is_empty(),
            "{args:?}: {}",
            stdout(&output)
        );
        assert_eq!(starts(&dir), Vec::<String>::new(), "{args:?}");
    }
}

#[test]
fn selecting_only_an_extension_rule_still_sends_it_the_files() {
    let dir = project("only", "select = [\"no-utils\"]\n");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    let output = lint(&dir, &["--select", "acme/by-sh"]);

    assert_eq!(
        findings(&output),
        ["app.py acme/by-sh"],
        "{}",
        stderr(&output)
    );
}

/// An extension that reports `notes.txt` only when it is sent the text of that file.
const READS_NOTES: &str = r#"while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"rules":[]}}' ;;
    *'"lint/files"'*)
      case "$line" in
        *'"path":"notes.txt","text":"x\n"'*)
          printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[{"path":"notes.txt","rule":"acme/by-sh","severity":"error","start":0,"end":1,"message":"read"}]}}' ;;
        *) printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[]}}' ;;
      esac ;;
    *'"shutdown"'*) printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":null}'; exit 0 ;;
  esac
done
"#;

#[test]
fn a_file_that_only_an_extension_reads_is_read_and_sent_with_its_text() {
    let dir = project("any-language", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    let how = format!("command = [\"sh\", \"{}/ext.sh\"]", folder("by-sh"));
    write(
        &dir,
        &format!("{}/rule.toml", folder("by-sh")),
        &rule_toml(
            "by-sh",
            "ACMESTRUC001",
            "",
            &implementation("command", "\"*\"", &how),
        ),
    );
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), READS_NOTES);
    write(&dir, "notes.txt", "x\n");

    let output = lint(&dir, &["--select", "acme/by-sh"]);

    assert_eq!(
        findings(&output),
        ["notes.txt acme/by-sh"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn with_the_cache_the_extension_runs_each_time_and_the_findings_are_the_same() {
    let dir = project("cached", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    let first = lint(&dir, &["--cache"]);
    let second = lint(&dir, &["--cache"]);

    assert_eq!(
        findings(&first),
        ["app.py acme/by-sh"],
        "{}",
        stderr(&first)
    );
    assert_eq!(findings(&second), findings(&first), "{}", stderr(&second));
    assert_eq!(starts(&dir).len(), 2);
}

#[test]
fn a_finding_of_another_rule_is_left_out_with_a_warning() {
    let dir = project("impostor", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/other-rule");
    add_rule(&dir, "other", "ACMESTRUC002", "", "acme/other");

    let output = lint(&dir, &[]);

    assert_eq!(findings(&output), ["app.py acme/other"]);
    let message = stderr(&output);
    assert!(
        message.contains("acme/by-sh") && message.contains("acme/other-rule"),
        "{message}"
    );
    assert_eq!(output.status.code(), Some(1), "{message}");
}

#[test]
fn a_finding_keeps_its_severity_unless_the_configuration_names_one() {
    let dir = project("severity", "");
    add_rule_with(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh", "info");

    let kept = lint(&dir, &[]);
    assert!(
        stdout(&kept).contains(" info [acme/by-sh]"),
        "{}",
        stdout(&kept)
    );
    assert_eq!(kept.status.code(), Some(0));

    let named = project(
        "severity-named",
        "[rules.\"acme/by-sh\"]\nseverity = \"error\"\n",
    );
    add_rule_with(&named, "by-sh", "ACMESTRUC001", "", "acme/by-sh", "info");
    let output = lint(&named, &[]);
    assert!(
        stdout(&output).contains(" error [acme/by-sh]"),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}
