//! The engine gives back the findings that a suppression channel silenced (ADR 0019 decision 3).

mod common;

use common::{engine, policy, summarize};

fn silenced(path: &str, text: &str) -> Vec<String> {
    summarize(&engine(policy()).lint_bytes(path, text.as_bytes()).silenced)
}

#[test]
fn a_finding_that_a_line_comment_silences_comes_back_apart() {
    let found = engine(policy()).lint_bytes("a.py", b"x = 1  # botica: ignore[flag-rule] FLAG\n");

    assert!(found.found.is_empty(), "{:?}", summarize(&found.found));
    assert_eq!(summarize(&found.silenced), ["flag-rule@1"]);
}

#[test]
fn a_finding_that_ignore_file_or_noqa_silences_comes_back_apart() {
    assert_eq!(
        silenced("a.py", "# botica: ignore-file[flag-rule]\nFLAG\nFLAG\n"),
        ["flag-rule@2", "flag-rule@3"]
    );
    assert_eq!(
        silenced("a.py", "x = 1  # noqa: BTFLG001\nFLAG  # noqa: BTFLG001\n"),
        ["flag-rule@2"]
    );
}

#[test]
fn a_finding_that_no_comment_silences_is_not_silenced() {
    let found = engine(policy()).lint_bytes("a.py", b"FLAG\n");

    assert_eq!(summarize(&found.found), ["flag-rule@1"]);
    assert!(found.silenced.is_empty());
}

#[test]
fn a_finding_about_the_comments_that_a_comment_silences_comes_back_apart() {
    let text = "x = 1  # botica: ignore[flag-rule, unused-suppression]\n";

    assert_eq!(silenced("a.py", text), ["unused-suppression@1"]);
}

#[test]
fn a_file_with_no_comment_has_nothing_silenced() {
    assert!(silenced("a.py", "FLAG\n").is_empty());
}
