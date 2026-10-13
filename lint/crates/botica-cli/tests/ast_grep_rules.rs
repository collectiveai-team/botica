//! The syntax rules of ast-grep are rules of the catalog: they are named, selected, ignored,
//! configured and suppressed like any other.

mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::{Path, PathBuf};

const PRINTS: &str = "print(\"hi\")\n";

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("ast-grep-rules", name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    write(&dir, "app.py", PRINTS);
    dir
}

/// The ids of the rules that reported, in the order of the output.
fn reported(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["lint", "."];
    all.extend(args);
    stdout(&command(dir, &all))
        .lines()
        .filter_map(|line| line.split_once('[')?.1.split_once(']'))
        .map(|(rule, _)| rule.to_owned())
        .collect()
}

#[test]
fn an_ast_grep_rule_runs_by_default_and_answers_to_its_slug_and_its_code() {
    let dir = project("named", "");

    assert_eq!(reported(&dir, &[]), ["log-no-print"]);
    assert_eq!(
        reported(&dir, &["--select", "log-no-print"]),
        ["log-no-print"]
    );
    assert_eq!(reported(&dir, &["--select", "BTLOG002"]), ["log-no-print"]);
}

#[test]
fn an_ast_grep_rule_can_be_ignored_and_a_narrow_selection_leaves_it_out() {
    let dir = project("ignored", "");

    assert!(reported(&dir, &["--ignore", "log-no-print"]).is_empty());
    assert!(reported(&dir, &["--ignore", "BTLOG002"]).is_empty());
    assert!(reported(&dir, &["--select", "no-utils"]).is_empty());
}

#[test]
fn the_configuration_ignores_an_ast_grep_rule_too() {
    let dir = project("configured", "ignore = [\"log-no-print\"]\n");

    assert!(reported(&dir, &[]).is_empty());
}

#[test]
fn an_ast_grep_rule_is_silenced_by_either_channel() {
    let dir = project("silenced", "");
    write(
        &dir,
        "app.py",
        "print(1)  # noqa: BTLOG002\nprint(2)  # botica: ignore[log-no-print]\nprint(3)\n",
    );

    let output = command(&dir, &["lint", "."]);

    let text = stdout(&output);
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.starts_with("app.py:3:"), "{text}");
}

#[test]
fn a_suppression_of_an_ast_grep_rule_that_is_ignored_is_non_enabled_not_unknown() {
    let dir = project("ignored-suppressed", "");
    write(&dir, "app.py", "print(1)  # botica: ignore[log-no-print]\n");

    let output = command(&dir, &["lint", ".", "--ignore", "log-no-print"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("non-enabled: `log-no-print`"), "{text}");
    assert!(!text.contains("Unknown rule"), "{text}");
}

#[test]
fn an_unused_suppression_of_an_ast_grep_rule_that_runs_is_reported() {
    let dir = project("unused", "");
    write(&dir, "app.py", "x = 1  # botica: ignore[log-no-print]\n");

    let text = stdout(&command(&dir, &["lint", "."]));

    assert!(text.contains("[unused-suppression]"), "{text}");
}

#[test]
fn rules_lists_an_ast_grep_rule_with_its_code_and_explain_describes_it() {
    let dir = project("listed", "");

    let listed = stdout(&command(&dir, &["rules"]));
    assert!(
        listed.contains("BTLOG002") && listed.contains("log-no-print"),
        "{listed}"
    );

    let output = command(&dir, &["explain", "log-no-print"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).contains("CES-46"), "{}", stdout(&output));
}

#[test]
fn no_astgrep_still_leaves_the_ast_grep_rules_out() {
    let dir = project("no-astgrep", "");

    assert!(reported(&dir, &["--no-astgrep"]).is_empty());
}
