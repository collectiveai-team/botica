//! `botica refs pin` turns a permalink of GitHub into a reference of a topic of the project.
//!
//! No test reaches the network: `https://github.com/acme/app.git` is a repository on disk
//! (`common/git_remote`).

mod common;

use common::external::{world, World, REPO};
use common::refs::{add_topic, failed, ok};

const TITLE_ONLY: &str = "title = \"E\"\n";

fn permalink(rev: &str, path: &str) -> String {
    format!("https://github.com/acme/app/blob/{rev}/{path}")
}

fn pin(world: &World, link: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec![
        "pin",
        link,
        "--topic",
        "demo",
        "--license",
        "MIT",
        "--note",
        "the second part",
    ];
    args.extend_from_slice(extra);
    world.run(REPO, &args)
}

fn topic_toml(world: &World) -> String {
    std::fs::read_to_string(world.dir.join("references/demo/topic.toml")).expect("the topic")
}

#[test]
fn pin_appends_the_lines_of_the_permalink_to_the_topic_with_the_first_line_as_anchor() {
    let world = world("pin-lines");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L2-L3", permalink(&world.first, "src/a.py"));

    let said = ok(&pin(&world, &link, &[]));

    let sha = &world.first;
    assert_eq!(
        topic_toml(&world),
        format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"{REPO}\"\nrev = \"{sha}\"\nlicense = \"MIT\"\n\
             path = \"src/a.py\"\nlines = \"2-3\"\nanchor = \"two\"\nnote = \"the second part\"\n"
        )
    );
    assert!(
        said.contains(&format!("{REPO}@{sha} src/a.py:2-3")),
        "{said}"
    );
    assert!(said.contains("demo"), "{said}");
}

#[test]
fn the_pinned_reference_is_what_show_prints() {
    let world = world("pin-show");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L2-L3", permalink(&world.first, "src/a.py"));
    ok(&pin(&world, &link, &[]));

    let text = ok(&world.run(REPO, &["show", "demo"]));

    assert!(text.contains("```\ntwo\nthree\n```\n"), "{text}");
}

#[test]
fn a_single_line_fragment_is_one_line() {
    let world = world("pin-one-line");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L3", permalink(&world.first, "src/a.py"));

    ok(&pin(&world, &link, &[]));

    let toml = topic_toml(&world);
    assert!(
        toml.contains("lines = \"3\"\nanchor = \"three\"\n"),
        "{toml}"
    );
}

#[test]
fn the_anchor_is_the_first_line_with_text_and_has_no_indentation() {
    let world = world("pin-anchor");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let rev = world
        .origin
        .commit(&[("src/b.py", b"\n    def f():\n        pass\n")]);
    let link = format!("{}#L1-L3", permalink(&rev, "src/b.py"));

    ok(&pin(&world, &link, &[]));

    let toml = topic_toml(&world);
    assert!(toml.contains("anchor = \"def f():\"\n"), "{toml}");
}

#[test]
fn a_permalink_without_lines_pins_the_whole_file_and_advises_the_lines() {
    let world = world("pin-whole");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = permalink(&world.first, "src/a.py");

    let output = pin(&world, &link, &[]);

    ok(&output);
    let toml = topic_toml(&world);
    assert!(!toml.contains("lines"), "{toml}");
    assert!(!toml.contains("anchor"), "{toml}");
    let advice = String::from_utf8_lossy(&output.stderr);
    assert!(
        advice.contains("warning: ") && advice.contains("points at the whole file"),
        "{advice}"
    );
}

#[test]
fn a_reference_is_added_after_the_ones_the_topic_has() {
    let world = world("pin-after");
    world.topic(REPO, "lines = \"1\"");
    let link = format!("{}#L2", permalink(&world.first, "src/a.py"));

    ok(&pin(&world, &link, &[]));

    let toml = topic_toml(&world);
    assert_eq!(toml.matches("[[references]]").count(), 2, "{toml}");
    assert!(toml.contains("lines = \"1\"\n\n[[references]]\n"), "{toml}");
}

#[test]
fn a_topic_that_does_not_exist_is_created_with_a_title() {
    let world = world("pin-create");
    let link = format!("{}#L2", permalink(&world.first, "src/a.py"));

    ok(&pin(&world, &link, &["--title", "A new topic"]));

    assert!(
        topic_toml(&world).starts_with("title = \"A new topic\"\n\n[[references]]\n"),
        "{}",
        topic_toml(&world)
    );
    let readme =
        std::fs::read_to_string(world.dir.join("references/demo/README.md")).expect("the README");
    assert_eq!(readme, "# A new topic\n");
    ok(&world.run(REPO, &["check"]));
}

#[test]
fn a_topic_that_does_not_exist_needs_a_title() {
    let world = world("pin-no-title");
    let link = format!("{}#L2", permalink(&world.first, "src/a.py"));

    let text = failed(&pin(&world, &link, &[]), 2);

    assert!(
        text.contains("no topic `demo`") && text.contains("--title"),
        "{text}"
    );
    assert!(!world.dir.join("references").exists());
}

#[test]
fn a_built_in_topic_cannot_take_a_reference() {
    let world = world("pin-builtin");
    let link = format!("{}#L2", permalink(&world.first, "src/a.py"));

    let output = world.run(
        REPO,
        &[
            "pin",
            &link,
            "--topic",
            "structured-logging",
            "--license",
            "MIT",
            "--note",
            "n",
        ],
    );

    let text = failed(&output, 2);
    assert!(text.contains("`structured-logging` is built in"), "{text}");
}

#[test]
fn a_branch_is_not_a_commit() {
    let world = world("pin-branch");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L2", permalink("main", "src/a.py"));

    let text = failed(&pin(&world, &link, &[]), 2);

    assert!(text.contains("full commit SHA"), "{text}");
    assert_eq!(topic_toml(&world), TITLE_ONLY);
}

#[test]
fn only_a_permalink_of_github_is_taken() {
    let world = world("pin-host");
    add_topic(&world.dir, "demo", TITLE_ONLY);

    let text = failed(
        &pin(
            &world,
            &format!("https://gitlab.com/acme/app/blob/{}/src/a.py", world.first),
            &[],
        ),
        2,
    );

    assert!(text.contains("github.com"), "{text}");
    let text = failed(&pin(&world, "src/a.py", &[]), 2);
    assert!(text.contains("github.com"), "{text}");
}

#[test]
fn a_fragment_that_is_not_lines_is_refused() {
    let world = world("pin-fragment");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let base = permalink(&world.first, "src/a.py");

    for fragment in [
        "#readme",
        "#L0",
        "#L3-L2",
        "#L2C1-L3C4",
        "#L",
        "#L2-3",
        "#L2-L",
    ] {
        let text = failed(&pin(&world, &format!("{base}{fragment}"), &[]), 2);
        assert!(text.contains("#L<line>"), "{fragment}: {text}");
    }
    assert_eq!(topic_toml(&world), TITLE_ONLY);
}

#[test]
fn lines_past_the_end_of_the_file_write_nothing() {
    let world = world("pin-past");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L2-L9", permalink(&world.first, "src/a.py"));

    let text = failed(&pin(&world, &link, &[]), 2);

    assert!(text.contains("past the end of the file"), "{text}");
    assert_eq!(topic_toml(&world), TITLE_ONLY);
}

#[test]
fn a_path_that_is_not_in_the_commit_writes_nothing() {
    let world = world("pin-missing");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = permalink(&world.first, "src/none.py");

    let text = failed(&pin(&world, &link, &[]), 2);

    assert!(text.contains("is not a file in"), "{text}");
    assert_eq!(topic_toml(&world), TITLE_ONLY);
}

#[test]
fn a_note_with_quotes_stays_one_valid_string() {
    let world = world("pin-quotes");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}#L2", permalink(&world.first, "src/a.py"));
    let mut args = vec!["pin", &link, "--topic", "demo", "--license", "MIT"];
    args.extend(["--note", "the \"two\" line\\n"]);

    ok(&world.run(REPO, &args));

    ok(&world.run(REPO, &["check"]));
    let text = ok(&world.run(REPO, &["show", "demo"]));
    assert!(text.contains("the \"two\" line\\n"), "{text}");
}

#[test]
fn the_query_of_the_address_is_left_out() {
    let world = world("pin-query");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("{}?plain=1#L2", permalink(&world.first, "src/a.py"));

    ok(&pin(&world, &link, &[]));

    let toml = topic_toml(&world);
    assert!(
        toml.contains("path = \"src/a.py\"\nlines = \"2\"\n"),
        "{toml}"
    );
}

#[test]
fn a_link_to_a_folder_or_a_tree_is_refused() {
    let world = world("pin-tree");
    add_topic(&world.dir, "demo", TITLE_ONLY);
    let link = format!("https://github.com/acme/app/tree/{}/src/a.py", world.first);

    let text = failed(&pin(&world, &link, &[]), 2);

    assert!(text.contains("is not a permalink of github.com"), "{text}");
}
