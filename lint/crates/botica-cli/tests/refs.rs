//! `botica refs` reads the topics of `references/` of a project.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{command, project_in, stderr, stdout, write};

const SOURCE: &str = "import os\n\ndef get_logger():\n    pass\n\nTAIL = 1\n";

const SHA: &str = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c";

/// A reference to a page of the web, to append to `topic_toml`. The host never resolves: a command
/// that fetched it would fail.
const PAGE: &str = "\n[[references]]\nurl = \"https://example.invalid/docs\"\n\
                    note = \"the documentation of the library\"\n";

fn topic_toml(extra: &str) -> String {
    format!(
        "title = \"Demo topic\"\n\n\
         [[references]]\npath = \"src/app.py\"\nlines = \"3-4\"\nanchor = \"def get_logger\"\n\
         note = \"the setup\"\n{extra}"
    )
}

fn add_topic(dir: &Path, id: &str, toml: &str) {
    write(dir, &format!("references/{id}/topic.toml"), toml);
    write(
        dir,
        &format!("references/{id}/README.md"),
        "Read `get_logger`.\n",
    );
}

/// A project with the file `src/app.py` and the topic `id`, which points at it.
fn project(name: &str, id: &str) -> PathBuf {
    let dir = project_in("refs", name, None);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, id, &topic_toml(""));
    dir
}

fn standard(dir: &Path, number: &str, references: &str) {
    write(
        dir,
        &format!("standards/CES/{number}-a-standard.md"),
        &format!(
            "---\nid: CES-{number}\ntitle: A standard\nreferences: {references}\n---\n\nText.\n"
        ),
    );
}

fn refs(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["refs"];
    all.extend_from_slice(args);
    command(dir, &all)
}

fn ok(output: &Output) -> String {
    assert!(output.status.success(), "{}", stderr(output));
    stdout(output)
}

fn failed(output: &Output, code: i32) -> String {
    assert_eq!(output.status.code(), Some(code), "{}", stdout(output));
    format!("{}{}", stdout(output), stderr(output))
}

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A catalog of the project, `acme`, whose rule `acme/no-eval` cites `references`.
fn acme_rule(dir: &Path, references: &str) {
    let root = ".botica/rules/acme";
    write(dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
             status = \"stable\"\nreferences = {references}\nscope = \"file\"\nneeds = \"syntax\"\n\
             run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n"
        ),
    );
    write(dir, &format!("{rule}/README.md"), README);
    write(
        dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: error\nmessage: \"ACMESTRUC001 (acme/no-eval): no\"\nrule:\n  pattern: eval($$$ARGS)\n",
    );
    write(dir, &format!("{rule}/fixtures/invalid/bad.py"), "eval(x)\n");
    write(dir, &format!("{rule}/fixtures/valid/ok.py"), "x = 1\n");
    write(
        dir,
        &format!("{rule}/fixtures/expected.json"),
        r#"{"diagnostics":[{"path":"bad.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":7,"line":1,"column":8}},"message":"ACMESTRUC001 (acme/no-eval): no"}]}"#,
    );
}

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
fn list_of_a_project_without_topics_says_so() {
    let dir = project_in("refs", "list-empty", None);

    let text = ok(&refs(&dir, &["list"]));

    assert!(text.contains("no topics"), "{text}");
}

#[test]
fn list_prints_the_address_of_a_page_reference_with_its_note() {
    let dir = project_in("refs", "list-page", None);
    add_topic(&dir, "demo", &topic_toml(PAGE));
    write(&dir, "src/app.py", SOURCE);

    let text = ok(&refs(&dir, &["list"]));

    assert!(
        text.contains("  https://example.invalid/docs  the documentation of the library\n"),
        "{text}"
    );
}

#[test]
fn show_prints_a_page_reference_as_its_address_and_its_note_without_a_code_block() {
    let dir = project_in("refs", "show-page", None);
    add_topic(
        &dir,
        "demo",
        "title = \"Demo topic\"\n\n[[references]]\nurl = \"https://example.invalid/docs\"\n\
         note = \"the documentation of the library\"\n",
    );

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(
        text.contains("## https://example.invalid/docs\n\nthe documentation of the library\n"),
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
        .find("## https://example.invalid/docs")
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
fn show_a_rule_whose_topic_is_not_in_this_project_names_the_topic() {
    let dir = project("show-missing-topic", "demo");

    let text = failed(&refs(&dir, &["show", "log-get-logger"]), 2);

    assert!(text.contains("structured-logging"), "{text}");
    assert!(text.contains("references/"), "{text}");
}

#[test]
fn show_an_external_reference_says_that_they_are_not_supported_yet() {
    let dir = project_in("refs", "show-external", None);
    add_topic(
        &dir,
        "ext",
        &format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"github.com/acme/app\"\nrev = \"{SHA}\"\n\
             license = \"MIT\"\npath = \"a.py\"\nnote = \"n\"\n"
        ),
    );

    let text = failed(&refs(&dir, &["show", "ext"]), 2);

    assert!(
        text.contains("external references are not supported yet"),
        "{text}"
    );
    assert!(
        text.contains(&format!("github.com/acme/app@{SHA} a.py")),
        "{text}"
    );
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
fn check_fails_on_an_external_reference_because_it_cannot_resolve_it_yet() {
    let dir = project_in("refs", "check-external", None);
    add_topic(
        &dir,
        "ext",
        &format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"github.com/acme/app\"\nrev = \"{SHA}\"\n\
             license = \"MIT\"\npath = \"a.py\"\nnote = \"n\"\n"
        ),
    );

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(
        text.contains("external references are not supported yet"),
        "{text}"
    );
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
