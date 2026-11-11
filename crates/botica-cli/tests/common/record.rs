//! What the tests of `botica lint --record` share: a project, a run with no variable of CI,
//! hook or key, and the lines of the history.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::{project_in, write};

pub const KEY: &str = "0123456789abcdef0123456789abcdef";

/// A file whose one finding, an unused suppression, is on its second line.
pub const UNUSED: &str = "import os\nx = 1  # botica: ignore[no-utils]\ny = 2\n";

/// The variables that decide the origin, the branch and the head of a run.
pub const DETECTED: [&str; 9] = [
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "PRE_COMMIT",
    "GITHUB_HEAD_REF",
    "GITHUB_EVENT_NAME",
    "GITHUB_EVENT_PATH",
    "CI_MERGE_REQUEST_SOURCE_BRANCH_SHA",
    "CI_MERGE_REQUEST_SOURCE_BRANCH_NAME",
    "CI_COMMIT_REF_NAME",
];

pub fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = project_in("lint-record", name, config);
    write(&dir, "a.py", UNUSED);
    dir
}

/// `botica lint` in `dir` with no variable of CI, hook or key, then those of `env`.
pub fn lint_with(dir: &Path, env: &[(&str, &str)], args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_botica"));
    command
        .arg("lint")
        .args(args)
        .arg("--no-astgrep")
        .current_dir(dir)
        .env_remove("BOTICA_HISTORY_KEY");
    for name in DETECTED {
        command.env_remove(name);
    }
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("run botica")
}

pub fn lint(dir: &Path, args: &[&str]) -> Output {
    lint_with(dir, &[], args)
}

pub fn history(dir: &Path) -> PathBuf {
    dir.join(".botica").join("history")
}

pub fn lines(dir: &Path, file: &str) -> Vec<Value> {
    let text = fs::read_to_string(history(dir).join(file)).unwrap_or_default();
    text.lines()
        .map(|line| serde_json::from_str(line).expect("each line is one JSON object"))
        .collect()
}

pub fn runs(dir: &Path) -> Vec<Value> {
    lines(dir, "runs.jsonl")
}

pub fn findings(dir: &Path) -> Vec<Value> {
    lines(dir, "findings.jsonl")
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8(output.stdout)
        .expect("UTF-8")
        .trim()
        .to_owned()
}

/// A project in a real repository, with `a.py` committed on the branch `topic`.
pub fn repository(name: &str) -> PathBuf {
    let dir = project(name, None);
    fs::remove_dir_all(dir.join(".git")).expect("remove the stand-in .git");
    git(&dir, &["init", "--quiet", "--initial-branch", "topic"]);
    git(&dir, &["add", "a.py"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    dir
}

pub fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}
