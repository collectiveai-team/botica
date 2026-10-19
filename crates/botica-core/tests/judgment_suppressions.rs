//! Suppressions of judgment rules: by file, with a reason, never unused.

mod common;

use botica_core::{Diagnostic, Severity};
use common::{engine, found, policy, summarize};

fn judged(text: &str) -> Vec<Diagnostic> {
    let policy = policy().with_judgment_rule("judged-rule", &["BTFLG005"]);
    engine(policy).lint_source("a.py", text)
}

#[test]
fn a_judgment_rule_is_suppressed_for_a_file_with_a_reason_and_is_never_unused() {
    let found = judged("# botica: ignore-file[judged-rule] -- the legacy importer\nx = 1\n");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_suppression_of_a_judgment_rule_without_a_reason_is_invalid() {
    for text in [
        "# botica: ignore-file[judged-rule]\nx = 1\n",
        "# botica: ignore-file[judged-rule] --  \nx = 1\n",
        "# botica: ignore-file[judged-rule] because\nx = 1\n",
    ] {
        let found = judged(text);
        assert_eq!(summarize(&found), ["invalid-suppression@1"], "{text:?}");
        assert_eq!(
            found[0].message,
            "A suppression of the judgment rule `judged-rule` needs a reason after `--`",
            "{text:?}"
        );
        assert_eq!(found[0].column, 23, "on the id: {text:?}");
    }
}

#[test]
fn a_judgment_rule_is_never_suppressed_on_a_line_or_with_noqa() {
    for (text, name) in [
        (
            "x = 1  # botica: ignore[judged-rule] -- a reason\n",
            "judged-rule",
        ),
        ("x = 1  # noqa: BTFLG005 -- a reason\n", "BTFLG005"),
    ] {
        let found = judged(text);
        assert_eq!(summarize(&found), ["invalid-suppression@1"], "{text:?}");
        assert_eq!(
            found[0].message,
            format!(
                "`{name}` is a judgment rule: suppress it for the whole file with \
                 `botica: ignore-file[judged-rule] -- <reason>`"
            ),
            "{text:?}"
        );
    }
}

#[test]
fn a_deterministic_rule_needs_no_reason() {
    assert!(found("a.py", "# botica: ignore-file[flag-rule]\nx = FLAG\n").is_empty());
}

fn judge_finding(rule: &str) -> Diagnostic {
    Diagnostic {
        path: "a.py".to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start: 0,
        end: 0,
        line: 2,
        column: 1,
        end_line: 2,
        end_column: 1,
        file_level: false,
        message: "judged".to_owned(),
    }
}

#[test]
fn silence_drops_the_findings_given_that_a_comment_of_the_file_suppresses_and_adds_nothing() {
    let policy = policy().with_judgment_rule("judged-rule", &["BTFLG005"]);
    let text = "# botica: ignore-file[BTFLG005] -- a reason\nx = 1  # botica: ignore[flag-rule]\n";
    let mut found = vec![judge_finding("judged-rule"), judge_finding("other-rule")];

    policy.silence("a.py", text, &mut found);

    assert_eq!(summarize(&found), ["other-rule@2"]);
}
