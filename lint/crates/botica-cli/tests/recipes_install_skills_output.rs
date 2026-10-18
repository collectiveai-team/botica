//! The CLI of skills writes why it failed to stdout and leaves stderr empty, with ANSI sequences
//! and a `■` before each error. `botica recipe install` takes the reason of a failed call from there.
#![cfg(unix)]

mod common;

use common::recipes::{add_recipe, project, with_npx};
use common::refs::failed;

const SKILL: &str = "[[skills]]\nsource = \"a/one\"\nnames = [\"x\"]\n";
const HEAD: &str = "●   opencode  Agent detected — installing non-interactively\n\x1b[?25l│\n";
const CLONE: &str = "◒  Cloning repository…\x1b[1G\x1b[J■  Failed to clone repository\x1b[?25h\n\
                     │  Failed to clone https://github.com/a/one.git: Cloning into '/x'...\n\
                     │  fatal: Remote branch v0.0.0-nope not found in upstream origin\n\
                     │  Tip: use the --yes (-y) and --global (-g) flags to install without prompts.\n\
                     └  Installation failed\n■  Canceled\n";
const NO_MATCH: &str = "◇  Found 38 skills\n│\n■  No matching skills found for: x\x1b[?25h\n\
                        ●  Available skills:\n│    - ask-matt\n│    - code-review\n";

/// What `botica recipe install` prints when a fake `npx` writes `stdout` and exits with 1.
fn install_failing(name: &str, stdout: &str, stderr: &str) -> String {
    let dir = project(name, None);
    add_recipe(&dir, "mine", SKILL);
    let script = format!("cat <<'EOF'\n{stdout}EOF\ncat >&2 <<'EOF'\n{stderr}EOF\nexit 1");
    failed(&with_npx(&dir, &script, &["install", "mine"]), 1)
}

fn reason(text: &str) -> &str {
    let line = text
        .lines()
        .find(|line| line.starts_with("skill x: failed: "))
        .unwrap_or_else(|| panic!("no failed line: {text}"));
    &line["skill x: failed: ".len()..]
}

#[test]
fn the_headline_of_the_error_in_stdout_is_the_reason() {
    let text = install_failing("out-no-match", &format!("{HEAD}{NO_MATCH}"), "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: No matching skills found for: x"
    );
}

#[test]
fn the_line_of_git_after_the_headline_says_why_a_clone_failed() {
    let text = install_failing("out-clone", &format!("{HEAD}{CLONE}"), "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: Failed to clone repository: \
         fatal: Remote branch v0.0.0-nope not found in upstream origin"
    );
}

#[test]
fn an_error_line_counts_like_a_fatal_one_and_the_last_of_them_wins() {
    let stdout = "■  Failed to clone repository\n│  fatal: first\n│  error: second\n│  Tip: x\n";

    let text = install_failing("out-error-line", stdout, "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: Failed to clone repository: error: second"
    );
}

#[test]
fn a_fatal_line_without_a_headline_is_the_reason() {
    let stdout = "│  fatal: no route  \n└  Installation failed\n";

    let text = install_failing("out-no-headline", stdout, "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: fatal: no route"
    );
}

#[test]
fn a_line_that_only_mentions_fatal_is_not_the_detail() {
    let stdout = "■  Failed to clone repository\n│  a fatal: in the middle\n│  an error: too\n";

    let text = install_failing("out-mention", stdout, "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: Failed to clone repository"
    );
}

#[test]
fn an_escape_sequence_ends_with_any_byte_from_at_to_tilde() {
    for (name, end) in [("out-esc-at", "\x1b[@"), ("out-esc-tilde", "\x1b[3~")] {
        let stdout = format!("■  Failed{end} to clone\n");

        let text = install_failing(name, &stdout, "");

        assert_eq!(
            reason(&text),
            "`npx skills add a/one` exited with code 1: Failed to clone"
        );
    }
}

#[test]
fn the_headline_is_the_first_error_that_is_not_canceled() {
    let stdout = "■  Canceled\n■  First\n■  Second\n";

    let text = install_failing("out-first", stdout, "");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: First"
    );
}

#[test]
fn stdout_without_a_reason_leaves_the_message_as_it_was() {
    for (name, stdout) in [
        ("out-canceled", "│\n■  Canceled\n"),
        (
            "out-noise",
            "└  Installation failed\n│  Tip: use the --yes flag\n",
        ),
        ("out-empty-headline", "■  \n"),
    ] {
        let text = install_failing(name, stdout, "");

        assert_eq!(reason(&text), "`npx skills add a/one` exited with code 1");
    }
}

#[test]
fn stderr_is_the_reason_when_it_has_text() {
    let text = install_failing("out-stderr", &format!("{HEAD}{CLONE}"), "  boom  \n");

    assert_eq!(
        reason(&text),
        "`npx skills add a/one` exited with code 1: boom"
    );
}
