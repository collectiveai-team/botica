//! Suppressions that name a rule of project scope: the configuration silences it, a comment is
//! invalid.

mod common;

use botica_core::Diagnostic;
use common::{engine, policy, summarize};

fn linted(text: &str) -> Vec<Diagnostic> {
    let policy = policy().with_project_rule("layered-rule", &["BTARCH900"]);
    engine(policy).lint_source("a.py", text)
}

#[test]
fn a_comment_that_names_a_project_rule_is_invalid_and_says_where_to_silence_it() {
    for (text, name) in [
        ("import x  # botica: ignore[layered-rule]\n", "layered-rule"),
        (
            "# botica: ignore-file[layered-rule]\nimport x\n",
            "layered-rule",
        ),
        ("import x  # noqa: BTARCH900\n", "BTARCH900"),
    ] {
        let found = linted(text);
        assert_eq!(summarize(&found), ["invalid-suppression@1"], "{text:?}");
        assert_eq!(
            found[0].message,
            format!(
                "`{name}` is a rule of the project and only the configuration silences it: \
                 name the path in `per-file-ignores`"
            ),
            "{text:?}"
        );
    }
}
