//! The git of the working directory, for `--diff` and `--staged`. Every path is relative to the
//! working directory, as the walk names the files.

use anyhow::{bail, Context, Result};
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Result<Output> {
    Command::new("git")
        .args(["-c", "core.quotePath=false"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .context("cannot run git, which `--diff` and `--staged` need")
}

fn git(args: &[&str]) -> Result<Vec<u8>> {
    checked(args, run(args)?)
}

fn checked(args: &[&str], output: Output) -> Result<Vec<u8>> {
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn text(args: &[&str]) -> Result<String> {
    String::from_utf8(git(args)?).context("git printed text that is not UTF-8")
}

/// The commit where `HEAD` and `rev` part.
pub(super) fn merge_base(rev: &str) -> Result<String> {
    let args = ["merge-base", rev, "HEAD"];
    let output = run(&args)?;
    // git exits with 1 and prints nothing when the two have no commit in common.
    if output.status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
        bail!(
            "`{rev}` and HEAD have no commit in common in this clone. A shallow clone, such as the \
             one of `actions/checkout` with its default `fetch-depth: 1`, lacks the history: fetch \
             it with `fetch-depth: 0` or `git fetch --deepen`"
        );
    }
    let base =
        String::from_utf8(checked(&args, output)?).context("git printed text that is not UTF-8")?;
    Ok(base.trim().to_owned())
}

/// The diff from `base` to the working tree, or from `HEAD` to the index when `base` is `None`,
/// without the deleted files and with a rename as a deletion and an addition.
pub(super) fn diff(base: Option<&str>) -> Result<String> {
    let mut args = vec![
        "diff",
        "--relative",
        "--no-color",
        "--no-ext-diff",
        "--no-renames",
        "--unified=0",
        "--diff-filter=d",
    ];
    match base {
        Some(base) => args.push(base),
        None => args.extend(["--cached", "HEAD"]),
    }
    text(&args)
}

/// The files under the working directory that git does not track and does not ignore.
pub(super) fn untracked() -> Result<Vec<String>> {
    let listed = text(&["ls-files", "--others", "--exclude-standard", "-z"])?;
    Ok(listed
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

/// The short sha and the message of each commit of `git log <range>`, newest first.
pub(super) fn commits(range: &str) -> Result<Vec<(String, String)>> {
    let listed = text(&["log", "--format=%h%x00%B%x1e", range, "--"])?;
    Ok(listed
        .split('\x1e')
        .filter_map(|entry| entry.trim_start_matches('\n').split_once('\0'))
        .map(|(sha, message)| (sha.to_owned(), message.to_owned()))
        .collect())
}

/// The bytes of `path` at `rev`, or in the index when `rev` is empty; `None` when it is not
/// there.
pub(super) fn show(rev: &str, path: &str) -> Option<Vec<u8>> {
    git(&["show", &format!("{rev}:./{path}")]).ok()
}
