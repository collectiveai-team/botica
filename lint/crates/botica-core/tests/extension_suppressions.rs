//! A rule that an extension runs after the walk: a comment may name it, and the engine cannot tell
//! whether the comment silences anything.

mod common;

use botica_core::suppress::Policy;
use common::{engine, policy, summarize};

fn lint(policy: Policy, text: &str) -> Vec<String> {
    summarize(&engine(policy).lint_source("a.py", text))
}

#[test]
fn a_comment_that_names_an_extension_rule_is_never_reported_as_unused() {
    let extension = policy().with_extension_rule("flag-rule", &["ACMEFLG001"]);

    assert!(lint(extension, "x = 1  # botica: ignore[flag-rule]\n").is_empty());
}

#[test]
fn a_comment_that_names_another_rule_is_reported_as_unused_as_ever() {
    assert_eq!(
        lint(policy(), "x = 1  # botica: ignore[flag-rule]\n"),
        ["unused-suppression@1"]
    );
}

#[test]
fn an_extension_rule_is_known_by_its_spellings() {
    let extension = policy()
        .with_code_prefix("ACME")
        .with_extension_rule("flag-rule", &["ACMEFLG001"]);

    assert!(lint(extension, "x = 1  # noqa: ACMEFLG001\n").is_empty());
}

#[test]
fn the_other_names_of_the_comment_are_still_checked() {
    let extension = policy().with_extension_rule("flag-rule", &["ACMEFLG001"]);

    assert_eq!(
        lint(
            extension,
            "x = 1  # botica: ignore[flag-rule, no-such-rule]\n"
        ),
        ["invalid-suppression@1"]
    );
}
