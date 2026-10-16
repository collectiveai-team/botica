mod common;

use common::{command, project_in, stdout};

fn explain(name: &str, config: Option<&str>, args: &[&str]) -> std::process::Output {
    let dir = project_in("explain", name, config);
    let mut all = vec!["explain"];
    all.extend_from_slice(args);
    command(&dir, &all)
}

fn text_of(output: &std::process::Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    stdout(output)
}

fn line_with<'a>(text: &'a str, label: &str) -> &'a str {
    text.lines()
        .find(|line| line.starts_with(label))
        .unwrap_or_else(|| panic!("no `{label}` line in:\n{text}"))
}

#[test]
fn says_what_the_rule_is_and_prints_its_readme() {
    let text = text_of(&explain("slug", None, &["no-utils"]));
    assert!(
        text.starts_with("no-utils (BTSTRUC002): No catch-all modules"),
        "{text}"
    );
    assert!(line_with(&text, "family").contains("structure"));
    assert!(line_with(&text, "status").contains("stable"));
    assert!(line_with(&text, "languages").contains("python"));
    assert!(line_with(&text, "standard").contains("CES-63"));
    assert!(text.contains("## Directive"), "{text}");
    assert!(
        text.contains("Name a module for what it holds."),
        "the README comes after the settings:\n{text}"
    );
}

#[test]
fn the_standard_line_names_the_standard_and_nothing_else() {
    let text = text_of(&explain("standard-line", None, &["no-utils"]));
    assert_eq!(line_with(&text, "standard:"), "standard:  CES-63");
}

#[test]
fn the_status_line_says_since_which_release_and_when_no_release_has_it_yet() {
    let released = text_of(&explain("released", None, &["no-utils"]));
    assert_eq!(
        line_with(&released, "status:"),
        "status:    stable (since 0.1.0)"
    );

    let dir = project_in("explain", "fresh", None);
    common::fresh_rule::lay_out(&dir);
    let fresh = text_of(&command(&dir, &["explain", "acme/no-eval"]));
    assert_eq!(
        line_with(&fresh, "status:"),
        "status:    preview (not released yet)"
    );
}

#[test]
fn a_rule_can_be_named_by_its_code() {
    let text = text_of(&explain("code", None, &["BTSTRUC001"]));
    assert!(text.starts_with("file-size-guard (BTSTRUC001)"), "{text}");
}

#[test]
fn shows_what_the_configuration_resolved_for_the_rule() {
    let text = text_of(&explain(
        "resolved",
        Some("[rules.file-size-guard]\nseverity = \"warning\"\nmax = 900\nextend-ignores = [\"docs/**\"]\n"),
        &["file-size-guard"],
    ));
    assert!(line_with(&text, "state").contains("selected by `default`"));
    assert!(line_with(&text, "severity").contains("warning"));
    assert!(line_with(&text, "options").contains("max = 900"));
    assert!(line_with(&text, "options").contains("warn = 400"));
    assert!(line_with(&text, "config").contains("botica.toml"));
    assert!(
        text.contains("  *.lock  (rule)") && text.contains("  docs/**  (config)"),
        "each ignore says where it is written:\n{text}"
    );
}

#[test]
fn says_why_a_rule_is_not_active() {
    let text = text_of(&explain(
        "inactive",
        None,
        &["no-utils", "--ignore", "BTSTRUC002"],
    ));
    let state = line_with(&text, "state");
    assert!(
        state.contains("not active") && state.contains("ignored by `BTSTRUC002`"),
        "{state}"
    );
}

#[test]
fn an_unknown_rule_is_a_usage_error() {
    let output = explain("unknown", None, &["no-such-rule"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("no-such-rule") && message.contains("botica rules --all"),
        "{message}"
    );
}
