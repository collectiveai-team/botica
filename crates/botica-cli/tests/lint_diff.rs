//! `botica lint --diff`, `--staged` and `--diff-file` (ADR 0009): the files of the change are
//! linted, and only the findings on the lines it adds stay.

mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG: &str = "select = [\"log-no-print\", \"file-size-guard\"]\n\
[rules.file-size-guard]\nwarn = 3\nmax = 100\n";

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A repository whose first commit holds `files`.
fn repository(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = project_in("lint-diff", name, Some(CONFIG));
    std::fs::remove_dir_all(dir.join(".git")).expect("remove the stand-in .git");
    git(&dir, &["init", "--quiet"]);
    for (path, text) in files {
        write(&dir, path, text);
    }
    git(&dir, &["add", "--all"]);
    git(&dir, &["commit", "--quiet", "--message", "first"]);
    dir
}

/// `path:line [rule]` of each finding, in the order of the output.
fn reported(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["lint"];
    all.extend(args);
    stdout(&command(dir, &all))
        .lines()
        .filter_map(|line| {
            let (place, finding) = line.split_once(": ")?;
            let (path, position) = place.split_once(':')?;
            let (row, _) = position.split_once(':')?;
            let rule = finding.split_once('[')?.1.split_once(']')?.0;
            Some(format!("{path}:{row} [{rule}]"))
        })
        .collect()
}

#[test]
fn a_diff_keeps_the_findings_on_the_lines_it_adds_and_in_the_files_git_does_not_track() {
    let dir = repository(
        "rev",
        &[("app.py", "print(1)\nx = 1\n"), ("old.py", "print(0)\n")],
    );
    write(&dir, "app.py", "print(1)\nx = 1\nprint(2)\n");
    write(&dir, "new.py", "print(3)\n");

    assert_eq!(
        reported(&dir, &["--diff", "HEAD"]),
        [
            "app.py:1 [file-size-guard]",
            "app.py:3 [log-no-print]",
            "new.py:1 [log-no-print]"
        ]
    );
    assert_eq!(
        reported(&dir, &["--diff", "HEAD", "new.py"]),
        ["new.py:1 [log-no-print]"]
    );
}

#[test]
fn staged_reads_the_files_as_they_are_staged() {
    let dir = repository("staged", &[("app.py", "x = 1\n")]);
    write(&dir, "app.py", "x = 1\nprint(2)\n");
    git(&dir, &["add", "app.py"]);
    write(&dir, "app.py", "x = 1\nprint(2)\nx = 3\nprint(4)\n");

    assert_eq!(reported(&dir, &["--staged"]), ["app.py:2 [log-no-print]"]);
}

#[test]
fn a_diff_file_gives_the_changes_of_a_patch() {
    let dir = repository("patch", &[("app.py", "print(1)\nx = 1\n")]);
    write(&dir, "app.py", "print(1)\nprint(2)\n");
    write(
        &dir,
        "change.diff",
        "--- a/app.py\n+++ b/app.py\n@@ -2 +2 @@\n-x = 1\n+print(2)\n",
    );

    assert_eq!(
        reported(&dir, &["--diff-file", "change.diff"]),
        ["app.py:2 [log-no-print]"]
    );
}

#[test]
fn the_three_inputs_exclude_each_other_and_a_revision_git_does_not_know_fails_the_run() {
    let dir = repository("errors", &[("app.py", "x = 1\n")]);

    let both = command(&dir, &["lint", "--diff", "HEAD", "--staged"]);
    assert_eq!(both.status.code(), Some(2), "{}", stderr(&both));

    let unknown = command(&dir, &["lint", "--diff", "no-such-rev"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(
        stderr(&unknown).contains("git merge-base"),
        "{}",
        stderr(&unknown)
    );
}
