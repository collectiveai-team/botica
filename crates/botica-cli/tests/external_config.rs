//! `external-config` in `botica lint`: the run checks the configuration of the active external
//! rules once for the whole project, whatever files it is given.

mod common;

use std::path::PathBuf;

use common::external_rule::{
    complexity_rule, LACKS, SELECTS_COMPLEXITY, WITHOUT_THE_HOOK, WITH_THE_HOOK,
};
use common::{botica, project_in, stdout, write};

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("external-config", name, Some(config));
    complexity_rule(&dir);
    dir
}

fn finding() -> String {
    format!("prek.toml:1:1: error [external-config] {LACKS}")
}

#[test]
fn a_file_of_configuration_without_the_fragment_of_an_active_external_rule_is_reported() {
    let dir = project("lacks", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains(&finding()), "{}", stdout(&output));
}

#[test]
fn a_file_with_the_fragment_has_no_finding() {
    let dir = project("holds", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITH_THE_HOOK);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn an_external_rule_that_is_not_active_or_the_rule_switched_off_reports_nothing() {
    for (name, config) in [
        ("inactive", "preview = true\n"),
        (
            "ignored",
            "preview = true\nextend-select = [\"acme/complexity\"]\nignore = [\"external-config\"]\n",
        ),
    ] {
        let dir = project(name, config);
        write(&dir, "prek.toml", WITHOUT_THE_HOOK);
        write(&dir, "a.py", "x = 1\n");

        let output = botica(&dir, &["a.py"]);

        assert_eq!(output.status.code(), Some(0), "{name}: {}", stdout(&output));
    }
}

#[test]
fn the_file_is_read_at_the_root_of_the_project_and_shown_from_where_botica_runs() {
    let dir = project("below", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    write(&dir, "src/app/a.py", "x = 1\n");

    let output = botica(&dir.join("src/app"), &["a.py"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("../../prek.toml:1:1: error [external-config] prek.toml lacks"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_rules_of_the_project_scope_check_the_external_rules_that_are_selected() {
    let dir = project("scope", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);

    let project = botica(&dir, &["--scope", "project"]);
    assert_eq!(project.status.code(), Some(1));
    assert!(
        stdout(&project).contains(&finding()),
        "{}",
        stdout(&project)
    );

    let file = botica(&dir, &["--scope", "file"]);
    assert_eq!(file.status.code(), Some(0), "{}", stdout(&file));
}
