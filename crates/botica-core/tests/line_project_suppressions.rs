//! A rule of project scope whose findings sit on a line, as `external-config-weakened`: a comment
//! with a reason on that line silences it, in the files of settings that the tools read.

mod common;

use botica_core::suppress::Policy;
use botica_core::{Diagnostic, Document, Severity};
use common::{engine, policy, summarize};

const RULE: &str = "external-config-weakened";

fn with_the_rule() -> Policy {
    policy().with_line_project_rule(RULE, &["BT005"])
}

/// A finding of the rule on the line of `value` in `text`.
fn weakened(path: &str, text: &str, value: &str) -> Diagnostic {
    let start = text.find(value).expect("the value is in the text");
    let document = Document::new(path, text);
    let (line, column) = document.position(start);
    let (end_line, end_column) = document.position(start + value.len());
    Diagnostic {
        path: path.to_owned(),
        rule: RULE.to_owned(),
        severity: Severity::Warning,
        start,
        end: start + value.len(),
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message: "weakened".to_owned(),
    }
}

/// `rule@line` of what the pass of the rules of project scope leaves of the finding on `value`.
fn after_the_project_pass(path: &str, text: &str, value: &str) -> Vec<String> {
    let mut found = vec![weakened(path, text, value)];
    with_the_rule().report_project(path, text, &mut found);
    summarize(&found)
}

#[test]
fn a_comment_with_a_reason_on_the_line_silences_the_finding_in_toml_and_yaml() {
    for (path, text, value) in [
        (
            "pyproject.toml",
            "[tool.pyrefly]\npreset = \"basic\"  # botica: ignore[external-config-weakened] -- we move to strict in Q1\n",
            "preset = \"basic\"",
        ),
        (
            "uv.toml",
            "exclude-newer = \"3 days\"  # botica: ignore[BT005] -- the mirror lags a week\n",
            "exclude-newer = \"3 days\"",
        ),
        (
            "pnpm-workspace.yaml",
            "minimumReleaseAge: 60  # botica: ignore[external-config-weakened] -- hotfix of a CVE\n",
            "minimumReleaseAge: 60",
        ),
        (
            ".cargo/config",
            "[registry]\nglobal-min-publish-age = \"1 day\"  # botica: ignore[external-config-weakened] -- r\n",
            "global-min-publish-age = \"1 day\"",
        ),
    ] {
        assert!(
            after_the_project_pass(path, text, value).is_empty(),
            "{path}"
        );
    }
}

#[test]
fn npm_reads_a_comment_after_a_value_that_is_not_quoted() {
    for comment in ["#", ";", " #", " ;"] {
        let text = format!(
            "min-release-age=3{comment} botica: ignore[external-config-weakened] -- the mirror lags\n"
        );
        assert!(
            after_the_project_pass(".npmrc", &text, "min-release-age=3").is_empty(),
            "{comment:?}"
        );
    }
    let quoted = "min-release-age=\"3 # botica: ignore[external-config-weakened] -- r\"\n";
    assert_eq!(
        after_the_project_pass(".npmrc", quoted, "min-release-age="),
        [format!("{RULE}@1")]
    );
}

#[test]
fn in_an_ini_file_of_pytest_a_comment_is_a_line_of_its_own() {
    let after_the_value =
        "[pytest]\nxfail_strict = false  # botica: ignore[external-config-weakened] -- r\n";
    assert_eq!(
        after_the_project_pass("pytest.ini", after_the_value, "xfail_strict = false"),
        [format!("{RULE}@2")]
    );
    for comment in ["#", ";"] {
        let text = format!(
            "[pytest]\n{comment} botica: ignore-file[external-config-weakened] -- r\nxfail_strict = false\n"
        );
        assert!(
            after_the_project_pass("tox.ini", &text, "xfail_strict = false").is_empty(),
            "{comment}"
        );
    }
}

#[test]
fn a_comment_that_silences_nothing_is_reported_after_the_project_pass() {
    let text =
        "[tool.uv]\nexclude-newer = \"7 days\"  # botica: ignore[external-config-weakened] -- r\n";
    let mut found = Vec::new();

    with_the_rule().report_project("pyproject.toml", text, &mut found);

    assert_eq!(summarize(&found), ["unused-suppression@2"]);
}

#[test]
fn the_walk_accepts_a_comment_with_a_reason_and_never_calls_it_unused() {
    let with_a_reason = "x = 1  # botica: ignore[external-config-weakened] -- r\n";
    assert!(
        summarize(&engine(with_the_rule()).lint_source("pyproject.toml", with_a_reason)).is_empty()
    );

    let without = "x = 1  # botica: ignore[external-config-weakened]\n";
    let found = engine(with_the_rule()).lint_source("pyproject.toml", without);
    assert_eq!(summarize(&found), ["invalid-suppression@1"]);
    assert_eq!(
        found[0].message,
        "A suppression of `external-config-weakened` needs a reason after `--`"
    );
}

#[test]
fn another_rule_of_project_scope_is_still_silenced_in_the_configuration_only() {
    let policy = with_the_rule().with_project_rule("external-config", &["BT004"]);
    let text = "x = 1  # botica: ignore[external-config] -- r\n";

    assert_eq!(
        summarize(&engine(policy).lint_source("pyproject.toml", text)),
        ["invalid-suppression@1"]
    );
}
