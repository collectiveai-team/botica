mod common;

use common::{botica, lines, stdout, write};
use std::path::PathBuf;

fn project(name: &str, config: Option<&str>) -> PathBuf {
    common::project_in("post-process", name, config)
}

const UNUSED: &str = "x = 1  # botica: ignore[no-utils]\n";

#[test]
fn an_override_sets_the_severity_of_the_rules_about_suppressions_for_some_paths() {
    let dir = project(
        "unused-severity",
        Some("[[overrides]]\nfiles = [\"legacy/**\"]\nseverity = { unused-suppression = \"error\" }\n"),
    );
    write(&dir, "legacy/a.py", UNUSED);
    write(&dir, "src/a.py", UNUSED);

    let output = botica(&dir, &["."]);

    let text = stdout(&output);
    assert!(
        text.contains("legacy/a.py:1:10: error [unused-suppression]"),
        "{text}"
    );
    assert!(
        text.contains("src/a.py:1:10: warning [unused-suppression]"),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1), "{text}");
}

#[test]
fn an_override_can_ignore_or_switch_off_the_rules_about_suppressions_for_some_paths() {
    let dir = project(
        "unused-off",
        Some(concat!(
            "[[overrides]]\nfiles = [\"tests/**\"]\nignore = [\"unused-suppression\"]\n",
            "[[overrides]]\nfiles = [\"vendor/**\"]\nseverity = { BT002 = \"off\" }\n",
        )),
    );
    for path in ["tests/a.py", "vendor/a.py", "src/a.py"] {
        write(&dir, path, UNUSED);
    }

    let text = stdout(&botica(&dir, &["."]));

    assert!(
        text.contains("src/a.py:1:10: warning [unused-suppression]"),
        "{text}"
    );
    assert!(
        !text.contains("tests/") && !text.contains("vendor/"),
        "{text}"
    );
}

#[test]
fn a_severity_named_in_the_configuration_applies_to_the_findings_of_a_native_rule() {
    let dir = project(
        "native-severity",
        Some(concat!(
            "[rules.file-size-guard]\nmax = 6\n",
            "[[overrides]]\nfiles = [\"tests/**\"]\nseverity = { file-size-guard = \"warning\" }\n",
        )),
    );
    write(&dir, "tests/big.py", &lines(7));
    write(&dir, "src/big.py", &lines(7));

    let output = botica(&dir, &["."]);

    let text = stdout(&output);
    assert!(
        text.contains("src/big.py:1:1: error [file-size-guard]"),
        "{text}"
    );
    assert!(
        text.contains("tests/big.py:1:1: warning [file-size-guard]"),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn a_rule_that_sets_its_own_severity_keeps_it_unless_the_configuration_names_one() {
    let dir = project(
        "native-own",
        Some("[rules.file-size-guard]\nwarn = 3\nmax = 6\n"),
    );
    write(&dir, "warn.py", &lines(4));
    write(&dir, "error.py", &lines(7));

    let text = stdout(&botica(&dir, &["."]));

    assert!(
        text.contains("warn.py:1:1: warning [file-size-guard]"),
        "{text}"
    );
    assert!(
        text.contains("error.py:1:1: error [file-size-guard]"),
        "{text}"
    );

    let dir = project(
        "native-named",
        Some("[rules.file-size-guard]\nwarn = 3\nmax = 6\nseverity = \"warning\"\n"),
    );
    write(&dir, "error.py", &lines(7));
    let output = botica(&dir, &["."]);
    assert!(
        stdout(&output).contains("error.py:1:1: warning [file-size-guard]"),
        "a named severity replaces the rule's own"
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn io_error_follows_the_configured_severity() {
    let dir = project(
        "io-severity",
        Some("[rules.io-error]\nseverity = \"warning\"\n"),
    );

    let output = botica(&dir, &["typo/"]);

    assert!(
        stdout(&output).contains("typo/:1:1: warning [io-error]"),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn io_error_does_not_depend_on_the_languages_of_the_configuration() {
    let dir = project("io-languages", Some("languages = [\"python\"]\n"));

    let output = botica(&dir, &["typo/"]);

    assert!(
        stdout(&output).contains("typo/:1:1: error [io-error]"),
        "{}",
        stdout(&output)
    );
    assert_eq!(output.status.code(), Some(1));
}
