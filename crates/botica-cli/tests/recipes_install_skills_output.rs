//! `botica recipe install` asks `npx skills add` for `--json` and takes from its answer which skills
//! were skipped and why a call failed. The CLI of skills writes the JSON to stdout, and its own log,
//! with ANSI sequences, to stderr.
#![cfg(unix)]

mod common;

use common::recipes::{add_recipe, project, with_npx};
use common::refs::failed;

const SKILLS: &str = "[[skills]]\nsource = \"a/one\"\nnames = [\"x\", \"y\"]\n";
const CLONE_FAILED: &str = "[\n  {\n    \"status\": \"failed\",\n    \"error\": \"Failed to clone \
    repository\\nFailed to clone https://github.com/a/one.git: Cloning into '/x'...\\nfatal: Remote \
    branch v0.0.0-nope not found in upstream origin\\n\"\n  }\n]\n";

/// What `botica recipe install` prints when a fake `npx` writes `stdout` and `stderr` and exits
/// with 1, for the skills `x` and `y` of `a/one`.
fn install_failing(name: &str, stdout: &str, stderr: &str) -> String {
    let dir = project(name, None);
    add_recipe(&dir, "mine", SKILLS);
    let script = format!("cat <<'EOF'\n{stdout}EOF\ncat >&2 <<'EOF'\n{stderr}EOF\nexit 1");
    failed(&with_npx(&dir, &script, &["install", "mine"]), 1)
}

fn reason<'a>(text: &'a str, skill: &str) -> &'a str {
    let prefix = format!("skill {skill}: failed: ");
    let line = text
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no failed line for {skill}: {text}"));
    &line[prefix.len()..]
}

#[test]
fn the_call_asks_for_json() {
    let dir = project("json-args", None);
    add_recipe(&dir, "mine", SKILLS);

    let output = with_npx(&dir, common::recipes::NPX_INSTALLS, &["install", "mine"]);

    assert!(output.status.success());
    assert_eq!(
        common::recipes::npx_calls(&dir),
        ["skills add a/one --agent opencode --yes --json --skill x y"]
    );
}

#[test]
fn a_failed_call_says_its_headline_and_the_fatal_line_of_git() {
    let text = install_failing("json-clone", CLONE_FAILED, "");

    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: Failed to clone repository: \
         fatal: Remote branch v0.0.0-nope not found in upstream origin"
    );
    assert_eq!(reason(&text, "y"), reason(&text, "x"));
}

#[test]
fn a_one_line_error_is_the_reason_and_the_last_fatal_or_error_line_wins() {
    let one = "[{\"status\":\"failed\",\"error\":\"  no route  \"}]\n";
    let text = install_failing("json-one-line", one, "");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: no route"
    );

    let many =
        "[{\"status\":\"failed\",\"error\":\"Head\\nfatal: first\\n  error: second\\nTip\"}]\n";
    let text = install_failing("json-many", many, "");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: Head: error: second"
    );

    let only_fatal = "[{\"status\":\"failed\",\"error\":\"fatal: alone\"}]\n";
    let text = install_failing("json-only-fatal", only_fatal, "");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: fatal: alone"
    );
}

#[test]
fn the_json_comes_before_stderr_and_stderr_is_used_without_it() {
    let text = install_failing("json-first", CLONE_FAILED, "  boom  \n");
    assert!(
        reason(&text, "x").ends_with("Failed to clone repository: fatal: Remote branch v0.0.0-nope not found in upstream origin"),
        "{text}"
    );

    let text = install_failing("json-none", "not json at all\n", "  boom  \n");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: boom"
    );

    let text = install_failing(
        "json-not-an-array",
        "{\"status\":\"failed\",\"error\":\"e\"}\n",
        "",
    );
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1"
    );

    let text = install_failing(
        "json-no-failure",
        "[{\"name\":\"z\",\"status\":\"installed\"}]\n",
        "",
    );
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1"
    );
}

#[test]
fn stderr_without_ansi_and_without_hints_is_the_fallback() {
    let stderr = "\x1b[32mwhat went wrong\x1b[0m\nhint: try again\n";

    let text = install_failing("json-ansi", "", stderr);

    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: what went wrong"
    );
}

#[test]
fn a_skill_that_the_answer_skips_says_why_and_the_others_are_added() {
    let dir = project("json-skipped", None);
    add_recipe(&dir, "mine", SKILLS);
    let script = "echo \"$*\" >> \"$PWD/.npx-calls\"\nmkdir -p .agents/skills/y\necho s > .agents/skills/y/SKILL.md\n\
        cat <<'EOF'\n[{\"name\":\"x\",\"status\":\"skipped\",\"reason\":\"No matching skill found in source\"},\
        {\"name\":\"y\",\"status\":\"installed\"}]\nEOF\nexit 1";

    let text = failed(&with_npx(&dir, script, &["install", "mine"]), 1);

    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` skipped it: No matching skill found in source"
    );
    assert!(text.contains("skill y: added\n"), "{text}");
    assert!(
        text.ends_with("mine: 1 added, 0 ok, 0 pending, 1 failed\n"),
        "{text}"
    );
}

#[test]
fn a_skipped_entry_without_a_reason_or_a_name_is_left_out() {
    let dir = project("json-skipped-bare", None);
    add_recipe(&dir, "mine", SKILLS);
    let script = "cat <<'EOF'\n[{\"name\":\"x\",\"status\":\"skipped\"},{\"status\":\"skipped\",\"reason\":\"r\"}]\nEOF\nexit 0";

    let text = failed(&with_npx(&dir, script, &["install", "mine"]), 1);

    assert!(
        text.contains("skill x: failed: not in `.agents/skills/` after `npx skills add a/one`\n"),
        "{text}"
    );
    assert!(
        text.contains("skill y: failed: not in `.agents/skills/` after `npx skills add a/one`\n"),
        "{text}"
    );
}

#[test]
fn only_a_failed_entry_gives_the_reason_of_the_call_whatever_comes_before_it() {
    let stdout =
        "[{\"name\":\"z\",\"status\":\"installed\"},{\"status\":\"failed\",\"error\":\"boom\"}]\n";

    let text = install_failing("json-after-installed", stdout, "");

    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: boom"
    );
}

#[test]
fn blank_lines_in_the_error_and_a_word_that_only_mentions_fatal_are_not_the_reason() {
    let blank = "[{\"status\":\"failed\",\"error\":\"\\n  \\nHead\\nfatal: x\\n\\n\"}]\n";
    let text = install_failing("json-blank", blank, "");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: Head: fatal: x"
    );

    let mention =
        "[{\"status\":\"failed\",\"error\":\"Head\\nit says fatal: inline\\nan error: too\"}]\n";
    let text = install_failing("json-mention", mention, "");
    assert_eq!(
        reason(&text, "x"),
        "`npx skills add a/one` exited with code 1: Head"
    );
}

#[test]
fn each_skipped_skill_gets_its_own_reason_and_only_a_skipped_entry_counts() {
    let dir = project("json-two-skipped", None);
    add_recipe(&dir, "mine", SKILLS);
    let script = "cat <<'EOF'\n[{\"name\":\"x\",\"status\":\"skipped\",\"reason\":\"rx\"},\
        {\"name\":\"y\",\"status\":\"installed\",\"reason\":\"ry\"}]\nEOF\nexit 1";

    let text = failed(&with_npx(&dir, script, &["install", "mine"]), 1);

    assert_eq!(reason(&text, "x"), "`npx skills add a/one` skipped it: rx");
    assert_eq!(
        reason(&text, "y"),
        "`npx skills add a/one` exited with code 1"
    );

    let script = "cat <<'EOF'\n[{\"name\":\"y\",\"status\":\"skipped\",\"reason\":\"ry\"},\
        {\"name\":\"x\",\"status\":\"skipped\",\"reason\":\"rx\"}]\nEOF\nexit 1";
    let text = failed(&with_npx(&dir, script, &["install", "mine"]), 1);
    assert_eq!(reason(&text, "x"), "`npx skills add a/one` skipped it: rx");
    assert_eq!(reason(&text, "y"), "`npx skills add a/one` skipped it: ry");
}

#[test]
fn an_escape_sequence_in_stderr_ends_with_any_byte_from_at_to_tilde() {
    for (name, end) in [("json-esc-at", "\x1b[@"), ("json-esc-tilde", "\x1b[3~")] {
        let stderr = format!("what{end} went wrong\n");

        let text = install_failing(name, "", &stderr);

        assert_eq!(
            reason(&text, "x"),
            "`npx skills add a/one` exited with code 1: what went wrong"
        );
    }
}
