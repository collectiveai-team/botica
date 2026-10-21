//! `unused-suppression` reports a comment that names an extension rule and silences nothing, as it
//! does for the rules that botica runs itself.

mod common;

use common::extension::{add_rule, folder, implementation, lint, project, rule_toml};
use common::{stderr, stdout, write};

/// The lines of the output about suppression comments.
fn about_comments(output: &std::process::Output) -> Vec<String> {
    stdout(output)
        .lines()
        .filter(|line| line.contains("[unused-suppression]"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_comment_that_silences_a_finding_of_an_extension_rule_is_used() {
    let dir = project("suppress-used", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, "app.py", "x = 1  # botica: ignore[acme/by-sh]\n");

    let output = lint(&dir, &[]);

    assert_eq!(stdout(&output), "", "{}", stderr(&output));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn a_comment_on_a_line_without_a_finding_of_the_extension_rule_is_unused() {
    let dir = project("suppress-unused", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(
        &dir,
        "app.py",
        "x = 1\ny = 2  # botica: ignore[ACMESTRUC001]\n",
    );

    let output = lint(&dir, &[]);

    assert_eq!(
        about_comments(&output),
        ["app.py:2:10: warning [unused-suppression] Unused suppression (unused: `ACMESTRUC001`)"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_comment_that_names_an_extension_rule_that_does_not_read_the_file_is_reported() {
    let dir = project("suppress-not-here", "");
    add_rule(
        &dir,
        "by-sh",
        "ACMESTRUC001",
        "ignores = [\"skip/**\"]",
        "acme/by-sh",
    );
    write(&dir, "skip/a.py", "x = 1  # botica: ignore[acme/by-sh]\n");

    let output = lint(&dir, &[]);

    assert_eq!(
        about_comments(&output),
        ["skip/a.py:1:10: warning [unused-suppression] Unused suppression (non-enabled: `acme/by-sh`)"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_comment_that_names_an_extension_rule_that_is_not_active_is_reported() {
    let dir = project("suppress-inactive", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, "app.py", "x = 1  # botica: ignore[acme/by-sh]\n");

    let output = lint(&dir, &["--ignore", "acme/by-sh"]);

    assert_eq!(
        about_comments(&output),
        ["app.py:1:10: warning [unused-suppression] Unused suppression (non-enabled: `acme/by-sh`)"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_comment_that_names_an_extension_rule_that_failed_is_not_reported() {
    let dir = project("suppress-failed", "");
    let how = "command = [\"sh\", \"-c\", \"exit 3\"]";
    write(
        &dir,
        &format!("{}/rule.toml", folder("broken")),
        &rule_toml(
            "broken",
            "ACMESTRUC001",
            "",
            &implementation("command", "\"python\"", how),
        ),
    );
    write(
        &dir,
        &format!("{}/README.md", folder("broken")),
        common::extension::README,
    );
    write(&dir, "app.py", "x = 1  # botica: ignore[acme/broken]\n");

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert_eq!(about_comments(&output), Vec::<String>::new());
}
