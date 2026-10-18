//! What `zizmor` 1.31.1 reports on the workflows of the recipes of CI, when all of them are
//! installed in one repository (`zizmor --strict-collection`, the command of `zizmor.yml`).
//!
//! It reported a `high` finding (`dangerous-triggers`) in `opencode.yml` and a `low` one
//! (`self-repository`) in `release.yml`.

mod common;

use common::workflows::{text, workflow};

const OPENCODE: &str = "files/github/workflows/opencode.yml";

#[test]
fn opencode_runs_only_for_the_comment_of_an_owner_a_member_or_a_collaborator() {
    let opencode = workflow("ci-opencode", OPENCODE);

    let condition = opencode["jobs"]["opencode"]["if"].as_str().expect("if");

    assert!(
        condition.contains(
            "contains(fromJSON('[\"OWNER\",\"MEMBER\",\"COLLABORATOR\"]'), github.event.comment.author_association)"
        ),
        "{condition}"
    );
    assert!(condition.contains("contains(github.event.comment.body, '/oc')"));
    assert!(condition.contains("contains(github.event.comment.body, '/opencode')"));
}

#[test]
fn the_guard_and_the_mention_must_both_hold() {
    let opencode = workflow("ci-opencode", OPENCODE);

    let condition = opencode["jobs"]["opencode"]["if"].as_str().expect("if");

    let guard = condition.find("author_association").expect("the guard");
    let mention = condition.find("'/oc'").expect("the mention");
    assert!(guard < mention);
    assert!(condition.contains("&&"), "{condition}");
}

#[test]
fn the_issue_comment_trigger_is_ignored_on_its_own_line_with_the_reason_above_it() {
    let source = text("ci-opencode", OPENCODE);
    let lines: Vec<&str> = source.lines().collect();

    let at = lines
        .iter()
        .position(|line| line.trim_start().starts_with("issue_comment:"))
        .expect("the trigger");

    assert!(
        lines[at].contains("# zizmor: ignore[dangerous-triggers]"),
        "{}",
        lines[at]
    );
    assert!(
        lines[..at]
            .iter()
            .any(|line| line.contains("author_association")),
        "the reason is not above the trigger"
    );
}

#[test]
fn the_release_calls_the_tests_of_its_own_repository_with_the_self_repository_syntax() {
    let release = workflow("ci-publish", "files/github/workflows/release.yml");

    assert_eq!(
        release["jobs"]["test"]["uses"].as_str(),
        Some("$/.github/workflows/tests.yml")
    );
}
