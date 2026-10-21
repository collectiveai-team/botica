//! `botica refs show` prints a file in a fence that its own backticks cannot close.

mod common;

use common::refs::{add_topic, ok, refs};
use common::{project_in, write};

/// `show demo` of a project whose topic points at `docs/guide.md`, which holds `guide`.
fn shown(name: &str, guide: &str) -> String {
    let dir = project_in("refs", name, None);
    write(&dir, "docs/guide.md", guide);
    add_topic(
        &dir,
        "demo",
        "title = \"T\"\n\n[[references]]\npath = \"docs/guide.md\"\nnote = \"the guide\"\n",
    );
    ok(&refs(&dir, &["show", "demo"]))
}

#[test]
fn a_file_with_a_block_of_code_is_fenced_with_four_backticks() {
    let guide = "# Guide\n\n```python\nx = 1\n```\n";

    let text = shown("fence-block", guide);

    assert!(text.contains(&format!("\n````\n{guide}````\n")), "{text}");
}

#[test]
fn the_fence_is_one_longer_than_the_longest_run_of_backticks_in_the_file() {
    let guide = "a\n`````\nb\n``\n";

    let text = shown("fence-run", guide);

    assert!(
        text.contains(&format!("\n``````\n{guide}``````\n")),
        "{text}"
    );
}

#[test]
fn inline_code_does_not_lengthen_the_fence() {
    let guide = "Call `get_logger` and ``x``.\n";

    let text = shown("fence-inline", guide);

    assert!(text.contains(&format!("\n```\n{guide}```\n")), "{text}");
    assert!(!text.contains("````"), "{text}");
}

#[test]
fn the_fence_depends_on_the_lines_that_are_shown_and_not_on_the_rest_of_the_file() {
    let dir = project_in("refs", "fence-lines", None);
    write(&dir, "docs/guide.md", "plain\n```\nx\n```\n");
    add_topic(
        &dir,
        "demo",
        "title = \"T\"\n\n[[references]]\npath = \"docs/guide.md\"\nlines = \"1\"\nnote = \"n\"\n",
    );

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(text.contains("\n```\nplain\n```\n"), "{text}");
    assert!(!text.contains("````"), "{text}");
}

#[test]
fn a_last_line_without_a_newline_does_not_swallow_the_closing_fence() {
    let text = shown("fence-no-newline", "one\ntwo");

    assert!(text.contains("\n```\none\ntwo\n```\n"), "{text}");
}
