mod common;

use botica_catalog::Reference;
use common::file;
use common::topics::{load, rejected, topic, EXTERNAL, INTERNAL, README, SHA};

#[test]
fn loads_a_topic_with_its_references_and_its_readme() {
    let loaded = load(vec![topic("structured-logging", INTERNAL)]).expect("loads");

    let found = loaded.topic("structured-logging").expect("the topic");
    assert_eq!(found.id, "structured-logging");
    assert_eq!(found.title, "Structured logging");
    assert_eq!(found.readme, README);
    let [reference] = found.references.as_slice() else {
        panic!("one reference expected");
    };
    assert_eq!(
        reference.path.as_deref(),
        Some("scaffolding/templates/snippets/core/logger.py")
    );
    assert_eq!(reference.url, None);
    assert!(!reference.is_page());
    assert_eq!(reference.line_range(), Some((18, 30)));
    assert_eq!(reference.anchor.as_deref(), Some("def get_logger"));
    assert_eq!(reference.note, "the setup");
    assert!(!reference.is_external());
}

#[test]
fn the_topics_come_in_the_order_of_their_ids() {
    let loaded = load(vec![topic("zeta", INTERNAL), topic("alpha", INTERNAL)]).expect("loads");

    let ids: Vec<&str> = loaded.topics().iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["alpha", "zeta"]);
    assert!(loaded.topic("nope").is_none());
}

#[test]
fn an_external_reference_names_a_repository_a_commit_and_a_license() {
    let loaded = load(vec![topic("structured-logging", EXTERNAL)]).expect("loads");

    let reference = &loaded
        .topic("structured-logging")
        .expect("topic")
        .references[0];
    assert!(reference.is_external());
    assert_eq!(reference.repo.as_deref(), Some("github.com/acme/app"));
    assert_eq!(reference.rev.as_deref(), Some(SHA));
    assert_eq!(reference.license.as_deref(), Some("MIT"));
    assert_eq!(reference.line_range(), None);
}

#[test]
fn rejects_bad_line_ranges() {
    for bad in ["0-3", "5-2", "a-b", "3-", "-3", "1-2-3", ""] {
        let toml = INTERNAL.replace("18-30", bad);
        let message = rejected(vec![topic("structured-logging", &toml)]);
        assert!(message.contains("`lines`"), "{bad}: {message}");
    }
}

#[test]
fn accepts_a_single_line_and_a_range() {
    for good in ["7", "7-7", "1-400"] {
        let toml = INTERNAL.replace("18-30", good);
        load(vec![topic("structured-logging", &toml)]).unwrap_or_else(|e| panic!("{good}: {e}"));
    }
    assert_eq!(
        load(vec![topic("t", &INTERNAL.replace("18-30", "7"))])
            .expect("loads")
            .topic("t")
            .expect("topic")
            .references[0]
            .line_range(),
        Some((7, 7))
    );
}

#[test]
fn rejects_a_topic_without_a_readme() {
    let message = rejected(vec![vec![file("structured-logging/topic.toml", INTERNAL)]]);
    assert!(
        message.contains("structured-logging/README.md"),
        "{message}"
    );
    assert!(message.contains("missing README.md"), "{message}");
}

#[test]
fn rejects_a_readme_without_a_topic_toml() {
    let message = rejected(vec![vec![file("structured-logging/README.md", README)]]);
    assert!(message.contains("missing topic.toml"), "{message}");
}

#[test]
fn rejects_an_id_that_is_not_kebab_case() {
    let message = rejected(vec![topic("Structured_Logging", INTERNAL)]);
    assert!(message.contains("kebab-case"), "{message}");
}

#[test]
fn rejects_an_unknown_field_and_a_missing_note() {
    let message = rejected(vec![topic(
        "structured-logging",
        &format!("{INTERNAL}colour = \"red\"\n"),
    )]);
    assert!(message.contains("colour"), "{message}");

    let message = rejected(vec![topic(
        "structured-logging",
        &INTERNAL.replace("note = \"the setup\"\n", ""),
    )]);
    assert!(message.contains("note"), "{message}");
}

#[test]
fn rejects_a_topic_without_a_title_and_an_empty_path() {
    let message = rejected(vec![topic(
        "structured-logging",
        &INTERNAL.replace("title = \"Structured logging\"\n", ""),
    )]);
    assert!(message.contains("title"), "{message}");

    let message = rejected(vec![topic(
        "structured-logging",
        &INTERNAL.replace("scaffolding/templates/snippets/core/logger.py", ""),
    )]);
    assert!(message.contains("`path` is empty"), "{message}");
}

#[test]
fn rejects_a_file_that_is_not_part_of_a_topic() {
    let message = rejected(vec![vec![file("structured-logging/notes.txt", "x")]]);
    assert!(message.contains("unexpected file"), "{message}");
}

#[test]
fn reports_every_problem_at_once() {
    let message = rejected(vec![
        topic("one", &INTERNAL.replace("18-30", "0-1")),
        topic("two", &INTERNAL.replace("18-30", "0-1")),
    ]);
    assert!(message.contains("one/topic.toml"), "{message}");
    assert!(message.contains("two/topic.toml"), "{message}");
}

fn reference(lines: Option<&str>, anchor: Option<&str>) -> Reference {
    let mut toml =
        String::from("title = \"t\"\n[[references]]\npath = \"src/a.py\"\nnote = \"n\"\n");
    if let Some(lines) = lines {
        toml.push_str(&format!("lines = \"{lines}\"\n"));
    }
    if let Some(anchor) = anchor {
        toml.push_str(&format!("anchor = \"{anchor}\"\n"));
    }
    load(vec![topic("t", &toml)])
        .expect("loads")
        .topic("t")
        .expect("topic")
        .references[0]
        .clone()
}

const SOURCE: &str = "one\ntwo\ndef get_logger():\n    pass\nfive\n";

#[test]
fn a_reference_that_resolves_checks_clean() {
    assert_eq!(
        reference(Some("3-4"), Some("def get_logger")).check(SOURCE),
        Ok(())
    );
    assert_eq!(reference(None, None).check(SOURCE), Ok(()));
    assert_eq!(reference(Some("5"), None).check(SOURCE), Ok(()));
}

#[test]
fn lines_past_the_end_of_the_file_fail_the_check() {
    let message = reference(Some("4-9"), None)
        .check(SOURCE)
        .expect_err("fails");
    assert!(message.contains("4-9"), "{message}");
    assert!(message.contains("5 lines"), "{message}");
}

#[test]
fn an_anchor_outside_the_lines_fails_the_check() {
    let message = reference(Some("1-2"), Some("def get_logger"))
        .check(SOURCE)
        .expect_err("fails");
    assert!(message.contains("def get_logger"), "{message}");
    assert!(message.contains("1-2"), "{message}");
}

#[test]
fn an_anchor_without_lines_is_searched_in_the_whole_file() {
    assert_eq!(reference(None, Some("five")).check(SOURCE), Ok(()));
    assert!(reference(None, Some("six")).check(SOURCE).is_err());
}

#[test]
fn the_excerpt_is_the_lines_of_the_reference_or_the_whole_file() {
    assert_eq!(
        reference(Some("3-4"), None).excerpt(SOURCE),
        "def get_logger():\n    pass\n"
    );
    assert_eq!(reference(Some("5"), None).excerpt(SOURCE), "five\n");
    assert_eq!(reference(None, None).excerpt(SOURCE), SOURCE);
}
