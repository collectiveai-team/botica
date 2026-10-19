//! `botica rules eval <rule> [paths]`: what one deterministic rule finds in real code, for a
//! review of its false positives, whatever the configuration selects.

mod common;

use common::judgment::project;
use common::{command, stderr, stdout, write};

#[test]
fn eval_lists_what_the_rule_finds_even_when_the_configuration_turns_it_off() {
    let dir = project(
        "rules-eval",
        "off",
        "ignore = [\"acme/no-eval\"]\n[rules.\"acme/no-eval\"]\nenabled = false\nseverity = \"error\"\n",
    );
    write(&dir, "lib/other.py", "y = eval(x)\nz = 1\n");
    write(&dir, "app.py", "x = eval(\"1\")\n");

    let output = command(&dir, &["rules", "eval", "acme/no-eval", "lib"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "lib/other.py:1:5: error [acme/no-eval] ACMEQUAL002 (acme/no-eval): do not call eval\n"
    );
}

#[test]
fn eval_runs_that_rule_alone() {
    let dir = project("rules-eval", "alone", "");
    write(&dir, "utils.py", "x = eval(\"1\")\n");

    let output = command(&dir, &["rules", "eval", "ACMEQUAL002", "."]);

    let text = stdout(&output);
    assert!(text.contains("[acme/no-eval]"), "{text}");
    assert!(!text.contains("no-utils"), "{text}");
}

#[test]
fn eval_refuses_a_judgment_rule_and_an_unknown_one() {
    let dir = project("rules-eval", "refused", "");

    let judged = command(&dir, &["rules", "eval", "acme/judge-it"]);
    assert_eq!(judged.status.code(), Some(2));
    assert!(
        stderr(&judged).contains("a judge evaluates `acme/judge-it`"),
        "{}",
        stderr(&judged)
    );

    let unknown = command(&dir, &["rules", "eval", "no-such-rule"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(
        stderr(&unknown).contains("no-such-rule"),
        "{}",
        stderr(&unknown)
    );
}
