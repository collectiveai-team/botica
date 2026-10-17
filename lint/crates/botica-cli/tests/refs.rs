//! `botica refs` reads the topics of `references/` of a project.

mod common;

use common::refs::{
    acme_rule, add_topic, failed, ok, project, refs, standard, topic_toml, PAGE, SOURCE,
};
use common::{project_in, write};

#[test]
fn list_prints_each_topic_with_its_references_and_who_cites_it() {
    let dir = project("list", "structured-logging");
    standard(&dir, "74", "[\"structured-logging\"]");
    add_topic(&dir, "uncited", &topic_toml(""));

    let text = ok(&refs(&dir, &["list"]));

    let topic = text
        .split("\n\n")
        .find(|block| block.starts_with("structured-logging"))
        .unwrap_or_else(|| panic!("no block for the topic:\n{text}"));
    assert!(topic.contains("Demo topic"), "{topic}");
    assert!(topic.contains("src/app.py:3-4"), "{topic}");
    assert!(topic.contains("the setup"), "{topic}");
    assert!(topic.contains("log-get-logger"), "{topic}");
    assert!(topic.contains("log-no-print"), "{topic}");
    assert!(topic.contains("CES-74"), "{topic}");
    let uncited = text
        .split("\n\n")
        .find(|block| block.starts_with("uncited"))
        .unwrap_or_else(|| panic!("no block for the uncited topic:\n{text}"));
    assert!(uncited.contains("cited by: nothing"), "{uncited}");
}

#[test]
fn list_names_the_rules_of_the_catalogs_of_the_project_that_cite_a_topic() {
    let dir = project("list-project-rule", "demo");
    acme_rule(&dir, "[\"demo\"]");

    let text = ok(&refs(&dir, &["list"]));

    assert!(text.contains("acme/no-eval"), "{text}");
}

#[test]
fn list_of_a_project_without_topics_has_the_built_in_ones() {
    let dir = project_in("refs", "list-empty", None);

    let text = ok(&refs(&dir, &["list"]));

    assert!(text.starts_with("in-memory-adapters: "), "{text}");
    assert!(text.contains("\nstructured-logging: "), "{text}");
}

#[test]
fn list_prints_the_address_of_a_page_reference_with_its_note() {
    let dir = project_in("refs", "list-page", None);
    add_topic(&dir, "demo", &topic_toml(PAGE));
    write(&dir, "src/app.py", SOURCE);

    let text = ok(&refs(&dir, &["list"]));

    assert!(
        text.contains("  https://example.invalid/docs#setup  the documentation of the library\n"),
        "{text}"
    );
}

#[test]
fn show_prints_a_page_reference_as_its_address_and_its_note_without_a_code_block() {
    let dir = project_in("refs", "show-page", None);
    add_topic(
        &dir,
        "demo",
        "title = \"Demo topic\"\n\n[[references]]\nurl = \"https://example.invalid/docs#setup\"\n\
         note = \"the documentation of the library\"\n",
    );

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(
        text.contains(
            "## https://example.invalid/docs#setup\n\nthe documentation of the library\n"
        ),
        "{text}"
    );
    assert!(
        !text.contains("```"),
        "a page has no lines to show:\n{text}"
    );
    assert!(!text.contains("docs:"), "{text}");
}

#[test]
fn show_prints_the_pages_and_the_files_of_a_topic_in_the_order_of_its_file() {
    let dir = project_in("refs", "show-mixed", None);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, "demo", &topic_toml(PAGE));

    let text = ok(&refs(&dir, &["show", "demo"]));

    let file = text.find("## src/app.py:3-4").expect("the file reference");
    let page = text
        .find("## https://example.invalid/docs#setup")
        .expect("the page reference");
    assert!(file < page, "{text}");
}

#[test]
fn check_counts_a_page_reference_and_does_not_fetch_it() {
    let dir = project_in("refs", "check-page", None);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, "demo", &topic_toml(PAGE));

    let text = ok(&refs(&dir, &["check"]));

    assert_eq!(text, "1 topic and 2 references resolve\n");
}

#[test]
fn check_fails_on_the_docs_field_of_a_topic_that_no_longer_exists() {
    let dir = project("check-docs", "demo");
    add_topic(
        &dir,
        "old",
        "title = \"Old\"\ndocs = [\"https://example.com\"]\n",
    );

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("references/old/topic.toml"), "{text}");
    assert!(text.contains("docs"), "{text}");
}

#[test]
fn show_prints_the_readme_and_the_lines_of_each_reference() {
    let dir = project("show-topic", "demo");

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(text.contains("Demo topic"), "{text}");
    assert!(text.contains("Read `get_logger`."), "{text}");
    assert!(text.contains("src/app.py:3-4"), "{text}");
    assert!(text.contains("the setup"), "{text}");
    assert!(text.contains("def get_logger():\n    pass\n"), "{text}");
    assert!(
        !text.contains("import os"),
        "only the lines of the reference:\n{text}"
    );
    assert!(
        !text.contains("TAIL"),
        "only the lines of the reference:\n{text}"
    );
}

#[test]
fn show_a_rule_prints_the_topics_it_cites_by_slug_by_code_and_by_alias_of_the_catalog() {
    let dir = project("show-rule", "structured-logging");

    for key in ["log-get-logger", "BTLOG001"] {
        let text = ok(&refs(&dir, &["show", key]));
        assert!(text.contains("Demo topic"), "{key}: {text}");
        assert!(text.contains("def get_logger():"), "{key}: {text}");
    }
}

#[test]
fn show_a_standard_prints_the_topics_its_front_matter_cites() {
    let dir = project("show-standard", "demo");
    standard(&dir, "74", "[\"demo\"]");

    let text = ok(&refs(&dir, &["show", "CES-74"]));

    assert!(text.contains("Demo topic"), "{text}");
    assert!(text.contains("def get_logger():"), "{text}");
}

#[test]
fn show_a_rule_of_a_catalog_of_the_project_prints_its_topics() {
    let dir = project("show-project-rule", "demo");
    acme_rule(&dir, "[\"demo\"]");

    let text = ok(&refs(&dir, &["show", "acme/no-eval"]));

    assert!(text.contains("Demo topic"), "{text}");
}

#[test]
fn show_a_rule_that_cites_nothing_fails_and_says_so() {
    let dir = project("show-uncited", "demo");

    let text = failed(&refs(&dir, &["show", "no-utils"]), 2);

    assert!(text.contains("no-utils"), "{text}");
    assert!(text.contains("cites no topic"), "{text}");
}

#[test]
fn show_a_standard_that_cites_nothing_fails_and_says_so() {
    let dir = project("show-uncited-standard", "demo");
    standard(&dir, "74", "[]");

    let text = failed(&refs(&dir, &["show", "CES-74"]), 2);

    assert!(text.contains("CES-74 cites no topic"), "{text}");
}

#[test]
fn show_something_that_is_not_a_topic_a_rule_or_a_standard_fails() {
    let dir = project("show-unknown", "demo");

    let text = failed(&refs(&dir, &["show", "nothing-like-it"]), 2);

    assert!(
        text.contains("no topic, rule or standard `nothing-like-it`"),
        "{text}"
    );
}

#[test]
fn show_a_standard_whose_topic_is_not_in_this_project_names_the_topic() {
    let dir = project("show-missing-topic", "demo");
    standard(&dir, "74", "[\"ghost\"]");

    let text = failed(&refs(&dir, &["show", "CES-74"]), 2);

    assert!(text.contains("ghost"), "{text}");
    assert!(text.contains("references/"), "{text}");
}

#[test]
fn check_passes_when_every_reference_resolves_and_every_citation_has_a_topic() {
    let dir = project("check-ok", "demo");
    standard(&dir, "74", "[\"demo\"]");
    acme_rule(&dir, "[\"demo\"]");

    let text = ok(&refs(&dir, &["check"]));

    assert_eq!(text, "1 topic and 1 reference resolve\n");
}

#[test]
fn check_does_not_ask_a_project_for_the_topics_that_the_built_in_rules_cite() {
    let dir = project("check-built-in", "demo");

    ok(&refs(&dir, &["check"]));
}

#[test]
fn check_fails_when_the_file_of_a_reference_is_gone() {
    let dir = project("check-gone", "demo");
    std::fs::remove_file(dir.join("src/app.py")).expect("remove the file");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("references/demo/topic.toml"), "{text}");
    assert!(text.contains("src/app.py"), "{text}");
}

#[test]
fn check_fails_when_the_lines_are_past_the_end_of_the_file() {
    let dir = project("check-lines", "demo");
    write(&dir, "src/app.py", "one line\n");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("3-4"), "{text}");
    assert!(text.contains("1 lines"), "{text}");
}

#[test]
fn check_fails_when_the_anchor_is_not_in_the_lines() {
    let dir = project("check-anchor", "demo");
    write(&dir, "src/app.py", "a\nb\nc\nd\nanchor def get_logger\n");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("def get_logger"), "{text}");
}

#[test]
fn check_fails_when_the_path_leaves_the_project() {
    let dir = project("check-outside", "demo");
    add_topic(
        &dir,
        "outside",
        "title = \"O\"\n\n[[references]]\npath = \"../app.py\"\nnote = \"n\"\n",
    );

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("../app.py"), "{text}");
    assert!(text.contains("inside the project"), "{text}");
}

#[test]
fn check_fails_when_a_standard_cites_a_topic_that_does_not_exist() {
    let dir = project("check-standard", "demo");
    standard(&dir, "74", "[\"demo\", \"ghost\"]");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("standards/CES/74-a-standard.md"), "{text}");
    assert!(text.contains("ghost"), "{text}");
}

#[test]
fn check_fails_when_a_rule_of_a_catalog_of_the_project_cites_a_topic_that_does_not_exist() {
    let dir = project("check-project-rule", "demo");
    acme_rule(&dir, "[\"ghost\"]");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("acme/no-eval"), "{text}");
    assert!(text.contains("ghost"), "{text}");
}

#[test]
fn check_fails_when_the_references_of_a_standard_are_not_a_list_of_ids() {
    let dir = project("check-bad-front-matter", "demo");
    standard(&dir, "74", "demo");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("standards/CES/74-a-standard.md"), "{text}");
    assert!(text.contains("`references`"), "{text}");
}

#[test]
fn check_reports_a_topic_that_does_not_load() {
    let dir = project("check-load", "demo");
    write(
        &dir,
        "references/broken/topic.toml",
        "title = \"B\"\ncolour = 1\n",
    );
    write(&dir, "references/broken/README.md", "b\n");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("references/broken/topic.toml"), "{text}");
    assert!(text.contains("colour"), "{text}");
}

#[test]
fn check_reports_every_problem_in_one_run() {
    let dir = project("check-all", "demo");
    standard(&dir, "74", "[\"ghost\"]");
    write(&dir, "src/app.py", "one line\n");

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("ghost"), "{text}");
    assert!(text.contains("1 lines"), "{text}");
}

#[test]
fn the_index_of_the_standards_is_not_a_standard() {
    let dir = project("index", "demo");
    write(&dir, "standards/CES/index.md", "# Standards\n");

    ok(&refs(&dir, &["check"]));
}

#[test]
fn check_fails_when_the_path_is_absolute() {
    let dir = project("check-absolute", "demo");
    add_topic(
        &dir,
        "absolute",
        "title = \"A\"\n\n[[references]]\npath = \"/etc/hostname\"\nnote = \"n\"\n",
    );

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(text.contains("/etc/hostname"), "{text}");
    assert!(text.contains("inside the project"), "{text}");
}
