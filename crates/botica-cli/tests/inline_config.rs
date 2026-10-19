mod common;

use common::{botica, command, lines, project_in, stderr, stdout, write};
use std::path::PathBuf;

/// `utils.py` breaks `no-utils`; `big.py` breaks `file-size-guard`.
fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = project_in("inline-config", name, config);
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
fn a_setting_works_without_any_configuration_file() {
    let dir = project("no-file", None);
    let output = botica(&dir, &["--config", "select = [\"no-utils\"]"]);
    assert_eq!(rules_found(&output), ["no-utils"]);
}

#[test]
fn a_setting_wins_over_the_file_it_finds() {
    let dir = project("over-file", Some("ignore = [\"no-utils\"]\n"));
    assert_eq!(rules_found(&botica(&dir, &[])), ["file-size-guard"]);
    let output = botica(&dir, &["--config", "ignore = []"]);
    assert_eq!(rules_found(&output), ["no-utils", "file-size-guard"]);
}

#[test]
fn dotted_keys_reach_the_options_of_a_rule() {
    let dir = project("dotted", None);
    write(&dir, "small.py", &lines(6));
    let output = botica(
        &dir,
        &["small.py", "--config", "rules.file-size-guard.max = 5"],
    );
    assert!(
        stdout(&output).contains("limit is 5"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_file_given_by_path_is_the_base_and_settings_go_on_top() {
    let dir = project("by-path", Some("select = [\"no-utils\"]\n"));
    write(&dir, "other.toml", "select = [\"file-size-guard\"]\n");
    let output = botica(&dir, &["--config", "other.toml"]);
    assert_eq!(rules_found(&output), ["file-size-guard"]);

    write(&dir, "small.py", &lines(6));
    let output = botica(
        &dir,
        &[
            "small.py",
            "--config",
            "other.toml",
            "--config",
            "rules.file-size-guard.max = 5",
        ],
    );
    assert!(
        stdout(&output).contains("limit is 5"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_options_of_the_command_line_win_over_a_setting() {
    let dir = project("flags", None);
    let output = botica(
        &dir,
        &[
            "--config",
            "select = [\"no-utils\"]",
            "--select",
            "file-size-guard",
        ],
    );
    assert_eq!(rules_found(&output), ["file-size-guard"]);
}

#[test]
fn when_a_key_comes_twice_the_last_wins() {
    let dir = project("twice", None);
    let output = botica(
        &dir,
        &[
            "--config",
            "select = [\"no-utils\"]",
            "--config",
            "select = [\"file-size-guard\"]",
        ],
    );
    assert_eq!(rules_found(&output), ["file-size-guard"]);
}

#[test]
fn two_configuration_files_are_an_error() {
    let dir = project("two-files", None);
    write(&dir, "a.toml", "");
    write(&dir, "b.toml", "");
    let output = botica(&dir, &["--config", "a.toml", "--config", "b.toml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("more than one configuration file"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn what_is_neither_a_file_nor_a_valid_setting_is_a_usage_error_with_a_hint() {
    let dir = project("invalid", None);
    for (value, mentions) in [
        ("select = [no-utils]", "select"),
        ("no-such-key = 1", "no-such-key"),
        ("preview = \"yes\"", "preview"),
        ("missing.toml", "missing.toml"),
    ] {
        let output = botica(&dir, &["--config", value]);
        assert_eq!(output.status.code(), Some(2), "{value}");
        assert!(stdout(&output).is_empty(), "{value}");
        let message = stderr(&output);
        assert!(message.contains(mentions), "{value}: {message}");
        assert!(message.contains("key = value"), "{value}: {message}");
    }
}

#[test]
fn rules_and_explain_take_settings_too() {
    let dir = project("other-commands", None);
    let output = command(&dir, &["rules", "--json", "--config", "preview = true"]);
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    assert_eq!(document["config"]["preview"], true);

    let output = command(
        &dir,
        &["explain", "no-utils", "--config", "ignore = [\"no-utils\"]"],
    );
    assert!(
        stdout(&output).contains("not active, ignored by `no-utils`"),
        "{}",
        stdout(&output)
    );
}
