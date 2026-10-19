//! What the walk reaches: hidden directories, what `.gitignore` leaves out, and what
//! `extend-include` brings back.

mod common;

use common::{command, lines, project_in, stdout, write};
use std::path::{Path, PathBuf};

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("walk-scope", name, Some(config));
    std::fs::canonicalize(dir).expect("the project exists")
}

/// A file that `file-size-guard` reports wherever the walk reaches it.
fn big(dir: &Path, relative: &str) {
    write(dir, relative, &lines(701));
}

/// The files `file-size-guard` reports for these arguments, in the order of the output.
fn reported(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["lint", "--no-astgrep", "--select", "file-size-guard"];
    all.extend(args);
    stdout(&command(dir, &all))
        .lines()
        .filter(|line| line.contains("[file-size-guard]"))
        .filter_map(|line| line.split(':').next())
        .map(str::to_owned)
        .collect()
}

/// The files `file-size-guard` reports when the walk starts from `.`.
fn oversized(dir: &Path, extra: &[&str]) -> Vec<String> {
    let mut args = vec!["."];
    args.extend(extra);
    reported(dir, &args)
}

/// The files `file-size-guard` reports for files named one by one, as prek passes them.
fn oversized_named(dir: &Path, flags: &[&str], files: &[&str]) -> Vec<String> {
    let mut args = flags.to_vec();
    args.extend(files);
    reported(dir, &args)
}

#[test]
fn hidden_directories_are_walked_except_the_ones_of_tools() {
    let dir = project("hidden", "");
    big(&dir, ".github/scripts/ci.py");
    big(&dir, ".agents/skills/run.py");
    big(&dir, ".claude/hooks/guard.py");
    big(&dir, ".venv/lib/dep.py");
    big(&dir, ".mypy_cache/3.12/mod.py");
    big(&dir, ".git/hooks/pre-commit.py");
    big(&dir, "src/app.py");
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    for extra in [&[][..], &["--cache-dir", cache]] {
        assert_eq!(
            oversized(&dir, extra),
            [".github/scripts/ci.py", "src/app.py"],
            "{extra:?}"
        );
    }
}

#[test]
fn the_configuration_of_agents_is_left_out_unless_it_is_named_or_included() {
    let dir = project("agents", "");
    big(&dir, ".agents/skills/run.py");
    big(&dir, ".claude/hooks/guard.py");

    assert_eq!(oversized(&dir, &[]), Vec::<String>::new());
    assert_eq!(oversized(&dir, &[".agents"]), [".agents/skills/run.py"]);
    assert_eq!(oversized(&dir, &[".claude"]), [".claude/hooks/guard.py"]);

    let dir = project("agents-included", "extend-include = [\".claude/\"]\n");
    big(&dir, ".agents/skills/run.py");
    big(&dir, ".claude/hooks/guard.py");

    assert_eq!(oversized(&dir, &[]), [".claude/hooks/guard.py"]);
}

#[test]
fn extend_include_never_goes_into_dot_git() {
    let dir = project("include-dot-git", "extend-include = [\"*.py\"]\n");
    write(&dir, ".gitignore", "ignored/\n");
    big(&dir, ".git/hooks/pre-commit.py");
    big(&dir, "ignored/gen.py");

    assert_eq!(oversized(&dir, &[]), ["ignored/gen.py"]);
}

#[test]
fn a_hidden_directory_named_on_the_command_line_is_walked_even_if_it_is_a_tool_one() {
    let dir = project("named", "");
    big(&dir, ".venv/lib/dep.py");

    assert_eq!(oversized(&dir, &[".venv"]), [".venv/lib/dep.py"]);
}

#[test]
fn what_gitignore_leaves_out_stays_out() {
    let dir = project("gitignore", "");
    write(&dir, ".gitignore", "generated/\n");
    big(&dir, "generated/api.py");
    big(&dir, "src/app.py");

    assert_eq!(oversized(&dir, &[]), ["src/app.py"]);
}

#[test]
fn extend_include_brings_back_what_gitignore_or_a_tool_directory_left_out() {
    let dir = project(
        "include",
        "extend-include = [\"generated/api.py\", \".venv/lib/dep.py\"]\n",
    );
    write(&dir, ".gitignore", "generated/\n");
    big(&dir, "generated/api.py");
    big(&dir, "generated/other.py");
    big(&dir, ".venv/lib/dep.py");
    big(&dir, ".venv/lib/skipped.py");
    big(&dir, "src/app.py");

    assert_eq!(
        oversized(&dir, &[]),
        [".venv/lib/dep.py", "generated/api.py", "src/app.py"]
    );
}

#[test]
fn extend_include_takes_a_whole_directory_and_a_pattern_at_any_depth() {
    let dir = project(
        "include-kinds",
        "extend-include = [\"generated/\", \"*.gen.py\"]\n",
    );
    write(&dir, ".gitignore", "generated/\n*.gen.py\n");
    big(&dir, "generated/deep/api.py");
    big(&dir, "pkg/models.gen.py");
    write(&dir, "pkg/small.py", "x = 1\n");

    assert_eq!(
        oversized(&dir, &[]),
        ["generated/deep/api.py", "pkg/models.gen.py"]
    );
}

#[test]
fn a_file_that_the_walk_reaches_anyway_is_reported_once() {
    let dir = project("include-once", "extend-include = [\"src/*.py\"]\n");
    big(&dir, "src/app.py");
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    for extra in [&[][..], &["--cache-dir", cache]] {
        assert_eq!(oversized(&dir, extra), ["src/app.py"], "{extra:?}");
    }
}

/// How many files the run says it linted (`--timings`).
fn files_linted(dir: &Path, extra: &[&str]) -> usize {
    let mut args = vec!["lint", ".", "--no-astgrep", "--timings"];
    args.extend(extra);
    let output = command(dir, &args);
    let text = String::from_utf8(output.stderr).expect("UTF-8 output");
    let total = text
        .lines()
        .find(|line| line.starts_with("[timing] total"))
        .expect("a timing line");
    let before_files = total.split(" files").next().expect("a count");
    before_files
        .rsplit(' ')
        .next()
        .and_then(|count| count.parse().ok())
        .expect("a number")
}

#[test]
fn a_file_the_walk_reaches_and_extend_include_asks_for_is_linted_once() {
    let with = project("include-count", "extend-include = [\"src/*.py\"]\n");
    big(&with, "src/app.py");
    let without = project("include-count-none", "");
    big(&without, "src/app.py");
    let cache = with.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    assert_eq!(files_linted(&with, &[]), files_linted(&without, &[]));
    assert_eq!(
        files_linted(&with, &["--cache-dir", cache]),
        files_linted(&without, &[])
    );
}

#[test]
fn extend_exclude_wins_over_extend_include() {
    let dir = project(
        "include-excluded",
        "extend-include = [\"generated/\"]\nextend-exclude = [\"generated/private/\"]\n",
    );
    write(&dir, ".gitignore", "generated/\n");
    big(&dir, "generated/api.py");
    big(&dir, "generated/private/key.py");

    assert_eq!(oversized(&dir, &[]), ["generated/api.py"]);
}

#[test]
fn extend_include_works_behind_the_cache_too() {
    let dir = project("include-cache", "extend-include = [\"generated/api.py\"]\n");
    write(&dir, ".gitignore", "generated/\n");
    big(&dir, "generated/api.py");
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    assert_eq!(
        oversized(&dir, &["--cache-dir", cache]),
        ["generated/api.py"]
    );
}

const NAMED: [&str; 5] = [
    ".agents/skills/run.py",
    ".claude/hooks/guard.py",
    ".venv/lib/dep.py",
    "pkg/.claude/hooks/nested.py",
    "src/app.py",
];

fn project_with_named_files(name: &str, config: &str) -> PathBuf {
    let dir = project(name, config);
    for file in NAMED {
        big(&dir, file);
    }
    dir
}

#[test]
fn files_named_inside_a_tool_directory_are_checked_by_default() {
    let dir = project_with_named_files("named-tools", "");

    assert_eq!(oversized_named(&dir, &[], &NAMED), NAMED);
}

#[test]
fn force_exclude_leaves_out_files_named_inside_a_tool_directory() {
    let dir = project_with_named_files("force-tools", "");

    assert_eq!(
        oversized_named(&dir, &["--force-exclude"], &NAMED),
        ["src/app.py"]
    );
}

#[test]
fn force_exclude_leaves_out_a_tool_directory_named_on_the_command_line() {
    let dir = project_with_named_files("force-tools-dir", "");

    assert!(oversized_named(&dir, &["--force-exclude"], &[".claude", ".venv"]).is_empty());
}

#[test]
fn force_exclude_keeps_what_extend_include_asks_for_in_a_tool_directory() {
    let dir = project_with_named_files("force-tools-included", "extend-include = [\".claude/\"]\n");

    assert_eq!(
        oversized_named(&dir, &["--force-exclude"], &NAMED),
        [
            ".claude/hooks/guard.py",
            "pkg/.claude/hooks/nested.py",
            "src/app.py"
        ]
    );
}
