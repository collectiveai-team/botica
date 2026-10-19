//! `botica refs update` finds the anchor of each external reference at the head of its repository
//! and proposes the commit and the lines; `--write` applies them.
//!
//! No test reaches the network: `https://github.com/acme/app.git` is a repository on disk
//! (`common/git_remote`). The topic `demo` points at `src/a.py` at the first commit, which holds
//! `one`, `two` and `three`.

mod common;

use common::external::{world, World, REPO};
use common::refs::{failed, ok};

const SHIFTED: &[u8] = b"zero\nx\none\ntwo\nthree\n";

fn topic_toml(world: &World) -> String {
    std::fs::read_to_string(world.dir.join("references/demo/topic.toml")).expect("the topic")
}

fn head(world: &World, text: &[u8]) -> String {
    world.origin.commit(&[("src/a.py", text)])
}

#[test]
fn update_proposes_the_head_and_the_lines_where_the_anchor_moved() {
    let world = world("update-propose");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    let before = topic_toml(&world);
    let new = head(&world, SHIFTED);

    let text = ok(&world.run(REPO, &["update", "demo"]));

    assert!(
        text.contains(&format!(
            "references[0] {REPO}@{} src/a.py:2-3: rev -> {new}, lines 2-3 -> 4-5\n",
            world.first
        )),
        "{text}"
    );
    assert!(
        text.contains("demo: 1 to update, 0 up to date, 0 problems\n"),
        "{text}"
    );
    assert_eq!(
        topic_toml(&world),
        before,
        "without --write nothing is written"
    );
}

#[test]
fn write_rewrites_the_rev_and_the_lines_and_nothing_else() {
    let world = world("update-write");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    let new = head(&world, SHIFTED);
    let before = topic_toml(&world);

    let text = ok(&world.run(REPO, &["update", "demo", "--write"]));

    let expected = before
        .replace(&world.first, &new)
        .replace("lines = \"2-3\"", "lines = \"4-5\"");
    assert_eq!(topic_toml(&world), expected);
    assert!(
        text.contains("demo: 1 updated, 0 up to date, 0 problems\n"),
        "{text}"
    );
    ok(&world.run(REPO, &["check"]));
    let shown = ok(&world.run(REPO, &["show", "demo"]));
    assert!(shown.contains("```\ntwo\nthree\n```\n"), "{shown}");
}

#[test]
fn the_anchor_on_a_later_line_moves_the_whole_block() {
    let world = world("update-offset");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"three\"\n");
    head(&world, SHIFTED);

    let text = ok(&world.run(REPO, &["update", "demo"]));

    assert!(text.contains("lines 2-3 -> 4-5"), "{text}");
}

#[test]
fn a_reference_at_the_head_is_up_to_date() {
    let world = world("update-current");
    let head = head(&world, SHIFTED);
    world.topic_at(REPO, &head, "lines = \"4-5\"\nanchor = \"two\"\n");
    let before = topic_toml(&world);

    let text = ok(&world.run(REPO, &["update", "demo", "--write"]));

    assert!(text.contains("src/a.py:4-5: up to date\n"), "{text}");
    assert!(
        text.contains("demo: 0 updated, 1 up to date, 0 problems\n"),
        "{text}"
    );
    assert_eq!(topic_toml(&world), before);
}

#[test]
fn a_changed_text_at_the_same_lines_is_said() {
    let world = world("update-changed");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    head(&world, b"zero\ntwo\nTHREE\n");

    let text = ok(&world.run(REPO, &["update", "demo"]));

    assert!(text.contains("rev -> "), "{text}");
    assert!(text.contains(", the text changed\n"), "{text}");
    assert!(!text.contains("lines 2-3 ->"), "the lines stay: {text}");
}

#[test]
fn text_that_did_not_change_is_not_said_to_have_changed() {
    let world = world("update-same");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    head(&world, SHIFTED);

    let text = ok(&world.run(REPO, &["update", "demo"]));

    assert!(!text.contains("the text changed"), "{text}");
}

#[test]
fn an_anchor_that_is_gone_is_a_problem_and_writes_nothing() {
    let world = world("update-gone");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    let before = topic_toml(&world);
    let new = head(&world, b"nothing\nhere\n");

    let text = failed(&world.run(REPO, &["update", "demo", "--write"]), 1);

    assert!(
        text.contains(&format!("the anchor `two` is not in the file at {new}")),
        "{text}"
    );
    assert!(
        text.contains("demo: 0 updated, 0 up to date, 1 problem\n"),
        "{text}"
    );
    assert_eq!(topic_toml(&world), before);
}

#[test]
fn an_anchor_in_two_places_is_a_problem_that_names_the_lines() {
    let world = world("update-twice");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    head(&world, b"two\nx\ntwo\n");

    let text = failed(&world.run(REPO, &["update", "demo"]), 1);

    assert!(
        text.contains("the anchor `two` is in 2 places of the file (lines 1, 3)"),
        "{text}"
    );
}

#[test]
fn lines_without_an_anchor_cannot_be_followed() {
    let world = world("update-no-anchor");
    world.topic(REPO, "lines = \"2-3\"\n");
    head(&world, SHIFTED);

    let text = failed(&world.run(REPO, &["update", "demo"]), 1);

    assert!(text.contains("has `lines` and no `anchor`"), "{text}");
}

#[test]
fn a_block_that_would_start_before_the_file_is_a_problem() {
    let world = world("update-past");
    world.topic(REPO, "lines = \"1-3\"\nanchor = \"three\"\n");
    head(&world, b"x\nthree\n");

    let text = failed(&world.run(REPO, &["update", "demo"]), 1);

    assert!(text.contains("would start before the file"), "{text}");
}

#[test]
fn a_block_that_runs_past_the_end_of_the_file_is_a_problem() {
    let world = world("update-end");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"two\"\n");
    head(&world, b"x\ntwo\n");

    let text = failed(&world.run(REPO, &["update", "demo"]), 1);

    assert!(
        text.contains("would end past the end of the file, which has 2 lines"),
        "{text}"
    );
}

#[test]
fn a_reference_to_a_whole_file_takes_the_new_commit_only() {
    let world = world("update-whole");
    world.topic(REPO, "");
    let new = head(&world, SHIFTED);

    let text = ok(&world.run(REPO, &["update", "demo", "--write"]));

    let toml = topic_toml(&world);
    assert!(toml.contains(&format!("rev = \"{new}\"")), "{toml}");
    assert!(!toml.contains("lines"), "{toml}");
    assert!(text.contains(", the text changed\n"), "{text}");
}

#[test]
fn a_reference_to_a_whole_file_loses_nothing_when_its_anchor_is_gone() {
    let world = world("update-whole-gone");
    world.topic(REPO, "anchor = \"two\"\n");
    head(&world, b"nothing\n");

    let text = failed(&world.run(REPO, &["update", "demo"]), 1);

    assert!(
        text.contains("the anchor `two` is not in the file at"),
        "{text}"
    );
}

#[test]
fn references_that_are_not_external_files_are_left_alone() {
    let world = world("update-skip");
    world.topic(
        REPO,
        "lines = \"2-3\"\nanchor = \"two\"\n\n[[references]]\nurl = \"https://example.invalid/x#y\"\n\
         note = \"a page\"\n",
    );
    head(&world, SHIFTED);

    let text = ok(&world.run(REPO, &["update", "demo"]));

    assert!(text.contains("references[0]"), "{text}");
    assert!(!text.contains("references[1]"), "{text}");
    assert!(
        text.contains("demo: 1 to update, 0 up to date, 0 problems\n"),
        "{text}"
    );
}

#[test]
fn each_reference_is_followed_on_its_own() {
    let world = world("update-several");
    world.topic(
        REPO,
        "lines = \"2-3\"\nanchor = \"two\"\n\n[[references]]\nrepo = \"github.com/acme/app\"\n\
         rev = \"FIRST\"\nlicense = \"MIT\"\npath = \"src/a.py\"\nlines = \"1\"\n\
         anchor = \"absent\"\nnote = \"n\"\n",
    );
    let toml = topic_toml(&world).replace("FIRST", &world.first);
    std::fs::write(world.dir.join("references/demo/topic.toml"), toml).expect("write");
    let new = head(&world, SHIFTED);

    let text = failed(&world.run(REPO, &["update", "demo", "--write"]), 1);

    assert!(
        text.contains("demo: 1 updated, 0 up to date, 1 problem\n"),
        "{text}"
    );
    let toml = topic_toml(&world);
    assert!(toml.contains(&format!("rev = \"{new}\"")), "{toml}");
    assert!(
        toml.contains(&format!("rev = \"{}\"", world.first)),
        "{toml}"
    );
    assert!(text.contains("references[1]"), "{text}");
    assert!(
        text.contains("the anchor `absent` is not in lines 1-1"),
        "{text}"
    );
}

#[test]
fn an_unknown_topic_is_an_error() {
    let world = world("update-unknown");
    world.topic(REPO, "");

    let text = failed(&world.run(REPO, &["update", "nothing"]), 2);

    assert!(text.contains("no topic `nothing`"), "{text}");
}

#[test]
fn a_built_in_topic_cannot_be_written() {
    let world = world("update-builtin");

    let text = failed(
        &world.run_offline(&["update", "structured-logging", "--write"]),
        2,
    );

    assert!(text.contains("`structured-logging` is built in"), "{text}");
}

#[test]
fn a_repository_that_does_not_answer_is_an_error_that_names_it() {
    let world = world("update-offline");
    world.topic(
        "example.invalid/acme/app",
        "lines = \"2-3\"\nanchor = \"two\"\n",
    );

    let text = failed(&world.run_offline(&["update", "demo"]), 2);

    assert!(
        text.contains("cannot read the head of example.invalid/acme/app: "),
        "{text}"
    );
}

#[test]
fn references_written_as_an_inline_array_are_proposed_but_not_rewritten() {
    let world = world("update-inline");
    common::refs::add_topic(
        &world.dir,
        "demo",
        &format!(
            "title = \"E\"\nreferences = [{{ repo = \"{REPO}\", rev = \"{}\", license = \"MIT\", \
             path = \"src/a.py\", lines = \"2-3\", anchor = \"two\", note = \"n\" }}]\n",
            world.first
        ),
    );
    head(&world, SHIFTED);
    let before = topic_toml(&world);

    ok(&world.run(REPO, &["update", "demo"]));
    let text = failed(&world.run(REPO, &["update", "demo", "--write"]), 2);

    assert!(text.contains("cannot rewrite references[0]"), "{text}");
    assert_eq!(topic_toml(&world), before);
}

#[test]
fn only_the_table_of_the_reference_is_rewritten() {
    let world = world("update-tables");
    let second = world.first.clone();
    world.topic(
        REPO,
        &format!(
            "lines = \"2-3\"\nanchor = \"two\"\n\n[[references]]\nrepo = \"{REPO}\"\nrev = \"{second}\"\n\
             license = \"MIT\"\npath = \"src/a.py\"\nlines = \"1\"\nanchor = \"one\"\nnote = \"n\"\n"
        ),
    );
    let new = head(&world, SHIFTED);

    ok(&world.run(REPO, &["update", "demo", "--write"]));

    let toml = topic_toml(&world);
    assert_eq!(
        toml.matches(&format!("rev = \"{new}\"")).count(),
        2,
        "{toml}"
    );
    assert!(
        toml.contains("lines = \"4-5\"") && toml.contains("lines = \"3\""),
        "{toml}"
    );
}
