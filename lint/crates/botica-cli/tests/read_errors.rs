mod common;

use common::{command, lines, project_in, stderr, stdout, write};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn project(name: &str) -> PathBuf {
    let dir = project_in("read-errors", name, None);
    fs::canonicalize(dir).expect("the project exists")
}

/// A file nobody may read. `false` when this user reads it anyway (root), and the test has
/// nothing to say.
fn lock(dir: &Path, relative: &str) -> bool {
    write(dir, relative, "secret\n");
    let path = dir.join(relative);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod");
    let readable = fs::File::open(&path).is_ok();
    if readable {
        eprintln!("skipped: {relative} is readable by this user");
    }
    !readable
}

fn json(output: &std::process::Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).expect("a JSON document")
}

#[test]
fn a_file_the_walk_cannot_read_is_an_error_of_the_run_and_the_rest_is_still_reported() {
    let dir = project("walk");
    write(&dir, "big.txt", &lines(701));
    if !lock(&dir, "locked.txt") {
        return;
    }

    let output = command(
        &dir,
        &["lint", ".", "--no-astgrep", "--output-format", "json"],
    );

    let document = json(&output);
    let errors = document["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{document}");
    assert_eq!(errors[0]["kind"], "read");
    assert_eq!(errors[0]["path"], "locked.txt");
    assert!(
        errors[0]["message"]
            .as_str()
            .is_some_and(|m| m.contains("Permission denied")),
        "{document}"
    );
    assert_eq!(document["diagnostics"][0]["rule"], "file-size-guard");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn in_text_the_unreadable_file_is_said_on_stderr() {
    let dir = project("text");
    write(&dir, "big.txt", &lines(701));
    if !lock(&dir, "locked.txt") {
        return;
    }

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    assert!(stdout(&output).contains("[file-size-guard]"));
    assert!(
        stderr(&output).contains("read `locked.txt`"),
        "{}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_run_with_an_extension_says_it_too_next_to_the_extension_failure() {
    let dir = project("ext");
    write(&dir, "big.py", &lines(701));
    write(&dir, "ext.sh", "read -r line\nexit 0\n");
    if !lock(&dir, "locked.txt") {
        return;
    }

    let output = command(
        &dir,
        &[
            "lint",
            ".",
            "--no-astgrep",
            "--ext",
            "sh ext.sh",
            "--output-format",
            "json",
        ],
    );

    let document = json(&output);
    let kinds: Vec<&str> = document["errors"]
        .as_array()
        .expect("errors")
        .iter()
        .filter_map(|error| error["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["read", "extension"], "{document}");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn the_cached_path_says_it_too() {
    let dir = project("cache");
    if !lock(&dir, "locked.txt") {
        return;
    }
    let cache = dir.join("cache");

    let output = command(
        &dir,
        &[
            "lint",
            ".",
            "--no-astgrep",
            "--cache-dir",
            cache.to_str().expect("UTF-8 path"),
            "--output-format",
            "json",
        ],
    );

    let document = json(&output);
    assert_eq!(document["errors"][0]["kind"], "read", "{document}");
    assert_eq!(document["errors"][0]["path"], "locked.txt");
    assert_eq!(output.status.code(), Some(2));
}
