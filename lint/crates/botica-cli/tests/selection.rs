mod common;

use common::{botica, lines, project_in, stdout, write};
use std::path::PathBuf;

/// A project whose `utils.py` breaks `no-utils` and whose `big.py` breaks `file-size-guard`.
fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = project_in("selection", name, config);
    write(&dir, "utils.py", "x = 1\n");
    write(&dir, "big.py", &lines(800));
    dir
}

fn rules_found(output: &std::process::Output) -> Vec<&'static str> {
    let text = stdout(output);
    ["no-utils", "file-size-guard"]
        .into_iter()
        .filter(|rule| text.contains(&format!("[{rule}]")))
        .collect()
}

#[test]
fn without_options_every_default_rule_runs() {
    let dir = project("none", None);
    assert_eq!(
        rules_found(&botica(&dir, &[])),
        ["no-utils", "file-size-guard"]
    );
}

#[test]
fn select_runs_only_the_rules_it_names() {
    let dir = project("select", None);
    assert_eq!(
        rules_found(&botica(&dir, &["--select", "no-utils"])),
        ["no-utils"]
    );
    assert_eq!(
        rules_found(&botica(&dir, &["--select", "BTSTRUC001"])),
        ["file-size-guard"]
    );
}

#[test]
fn select_wins_over_what_the_file_ignores() {
    let dir = project("select-over-file", Some("ignore = [\"no-utils\"]\n"));
    assert_eq!(rules_found(&botica(&dir, &[])), ["file-size-guard"]);
    assert_eq!(
        rules_found(&botica(&dir, &["--select", "no-utils"])),
        ["no-utils"]
    );
}

#[test]
fn ignore_switches_a_rule_off() {
    let dir = project("ignore", None);
    assert_eq!(
        rules_found(&botica(&dir, &["--ignore", "file-size-guard"])),
        ["no-utils"]
    );
}

#[test]
fn several_selectors_can_be_comma_separated_or_repeated() {
    let dir = project("several", None);
    let both = ["no-utils", "file-size-guard"];
    assert_eq!(
        rules_found(&botica(
            &dir,
            &["--select", "no-utils,file-size-guard", "--ignore", "BT001"]
        )),
        both
    );
    assert_eq!(
        rules_found(&botica(
            &dir,
            &["--select", "no-utils", "--select", "file-size-guard"]
        )),
        both
    );
}

#[test]
fn a_selector_that_names_nothing_is_a_usage_error_naming_the_option() {
    let dir = project("unknown", None);
    let output = botica(&dir, &["--select", "no-such-rule"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("--select") && message.contains("no-such-rule"),
        "{message}"
    );
}

#[test]
fn preview_is_accepted() {
    let dir = project("preview", None);
    let output = botica(&dir, &["--preview"]);
    assert_eq!(output.status.code(), Some(1), "findings, not a usage error");
}
