mod common;

use common::{botica, command, lines, project_in, stdout, write};
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
fn extend_select_adds_rules_to_a_select() {
    let dir = project("extend", Some("select = [\"no-utils\"]\n"));
    assert_eq!(rules_found(&botica(&dir, &[])), ["no-utils"]);
    assert_eq!(
        rules_found(&botica(&dir, &["--extend-select", "file-size-guard"])),
        ["no-utils", "file-size-guard"]
    );
    assert_eq!(
        rules_found(&botica(
            &dir,
            &["--select", "file-size-guard", "--extend-select", "no-utils"]
        )),
        ["no-utils", "file-size-guard"]
    );
}

#[test]
fn preview_and_no_preview_are_accepted_and_the_last_one_wins() {
    let dir = project("no-preview", None);
    for args in [
        &["--no-preview"][..],
        &["--preview", "--no-preview"],
        &["--no-preview", "--preview"],
    ] {
        assert_eq!(botica(&dir, args).status.code(), Some(1), "{args:?}");
    }
}

#[test]
fn preview_is_accepted() {
    let dir = project("preview", None);
    let output = botica(&dir, &["--preview"]);
    assert_eq!(output.status.code(), Some(1), "findings, not a usage error");
}

/// With the embedded syntax rules on (no `--no-astgrep`): does `log.py` get its `print` finding?
fn prints_are_reported(dir: &std::path::Path, args: &[&str]) -> bool {
    let mut all = vec!["lint"];
    all.extend_from_slice(args);
    stdout(&command(dir, &all)).contains("[log-no-print]")
}

#[test]
fn a_narrow_select_runs_only_what_it_names_so_the_embedded_syntax_rules_stay_off() {
    let dir = project("legacy", None);
    write(&dir, "log.py", "print('x')\n");
    assert!(prints_are_reported(&dir, &[]));
    assert!(!prints_are_reported(&dir, &["--select", "no-utils"]));
    assert!(!prints_are_reported(&dir, &["--select", "BTSTRUC"]));
}

#[test]
fn selecting_default_or_all_keeps_the_embedded_syntax_rules_on() {
    let dir = project("legacy-all", None);
    write(&dir, "log.py", "print('x')\n");
    assert!(prints_are_reported(&dir, &["--select", "default"]));
    assert!(prints_are_reported(&dir, &["--select", "all"]));
    assert!(prints_are_reported(&dir, &["--select", "no-utils,default"]));
}

#[test]
fn ignore_and_extend_select_do_not_switch_them_off() {
    let dir = project("legacy-ignore", None);
    write(&dir, "log.py", "print('x')\n");
    assert!(prints_are_reported(&dir, &["--ignore", "no-utils"]));
}
