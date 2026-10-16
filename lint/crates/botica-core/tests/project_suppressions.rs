//! Suppressions that name the rules of a catalog of a project: `acme/flag`, `ACMEFLG001`.

mod common;

use botica_core::suppress::Policy;
use common::{engine, policy, summarize};

/// `flag-rule` known as a rule of the catalog `acme`, whose codes start with `ACME`.
fn acme() -> Policy {
    policy()
        .with_code_prefix("ACME")
        .with_rule("flag-rule", &["acme/flag", "ACMEFLG001"])
}

fn lint(policy: Policy, text: &str) -> Vec<String> {
    summarize(&engine(policy).lint_source("a.py", text))
}

#[test]
fn a_noqa_comment_names_a_rule_of_a_project_by_its_code() {
    assert!(lint(acme(), "x = FLAG  # noqa: ACMEFLG001\n").is_empty());
    assert!(lint(acme(), "x = FLAG  # NOQA: E501, ACMEFLG001, F401\n").is_empty());
}

#[test]
fn a_botica_comment_names_a_rule_of_a_project_by_its_prefixed_id_or_its_code() {
    for id in ["acme/flag", "ACMEFLG001"] {
        let text = format!("x = FLAG  # botica: ignore[{id}]\n");
        assert!(lint(acme(), &text).is_empty(), "{id}");
    }
}

#[test]
fn without_the_code_of_the_catalog_a_noqa_code_belongs_to_another_tool() {
    let without = policy().with_rule("flag-rule", &["ACMEFLG001"]);

    assert_eq!(
        lint(without, "x = FLAG  # noqa: ACMEFLG001\n"),
        ["flag-rule@1"]
    );
}

#[test]
fn an_unknown_code_of_a_catalog_of_the_project_is_reported() {
    assert_eq!(
        lint(acme(), "x = 1  # noqa: ACMENOPE001\n"),
        ["invalid-suppression@1"]
    );
    assert_eq!(
        lint(acme(), "x = 1  # botica: ignore[acme/no-such-rule]\n"),
        ["invalid-suppression@1"]
    );
}

#[test]
fn the_code_of_a_catalog_does_not_make_every_code_botica_s() {
    assert!(lint(acme(), "x = 1  # noqa: E501, ZZ001, ACM1\n").is_empty());
}

#[test]
fn the_codes_of_the_built_in_catalog_still_name_its_rules() {
    assert!(lint(acme(), "x = FLAG  # noqa: BTFLG001\n").is_empty());
    assert_eq!(
        lint(acme(), "x = 1  # noqa: BTNOPE001\n"),
        ["invalid-suppression@1"]
    );
}

#[test]
fn an_unused_suppression_of_a_rule_of_a_project_is_reported() {
    assert_eq!(
        lint(acme(), "x = 1  # noqa: ACMEFLG001\n"),
        ["unused-suppression@1"]
    );
}
