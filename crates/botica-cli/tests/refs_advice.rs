//! A reference should point at a fragment: `botica refs check` advises, and does not fail, when it
//! points at a whole file or a whole page, and `show` and `export` carry the quote of a page.

mod common;

use std::path::{Path, PathBuf};

use common::refs::{add_topic, ok, refs, SOURCE};
use common::{project_in, write};
use serde_json::Value;

fn project(name: &str, toml: &str) -> PathBuf {
    let dir = project_in("refs-advice", name, None);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, "demo", toml);
    dir
}

const WHOLE_FILE: &str = "title = \"T\"\n\n[[references]]\npath = \"src/app.py\"\nnote = \"n\"\n";
const PAGE_URL: &str = "https://example.invalid/docs";

fn page(url: &str, extra: &str) -> String {
    format!("title = \"T\"\n\n[[references]]\nurl = \"{url}\"\nnote = \"n\"\n{extra}")
}

fn checked(dir: &Path) -> (String, String) {
    let output = refs(dir, &["check"]);
    assert!(output.status.success(), "advice is not a failure");
    (common::stdout(&output), common::stderr(&output))
}

#[test]
fn a_whole_file_is_advised_to_name_lines_and_the_check_still_passes() {
    let dir = project("whole-file", WHOLE_FILE);

    let (out, err) = checked(&dir);

    assert_eq!(out, "1 topic and 1 reference resolve, 1 warning\n");
    assert!(
        err.starts_with(
            "warning: references/demo/topic.toml: references[0] (src/app.py): points at the whole file"
        ),
        "{err}"
    );
}

#[test]
fn a_page_without_a_fragment_or_a_quote_is_advised() {
    let dir = project("whole-page", &page(PAGE_URL, ""));

    let (out, err) = checked(&dir);

    assert_eq!(out, "1 topic and 1 reference resolve, 1 warning\n");
    assert!(
        err.contains(&format!(
            "references[0] ({PAGE_URL}): points at the whole page"
        )),
        "{err}"
    );
}

#[test]
fn a_fragment_or_a_quote_or_lines_ends_the_advice() {
    for (name, toml) in [
        ("fragment", page(&format!("{PAGE_URL}#setup"), "")),
        ("quote", page(PAGE_URL, "quote = \"the passage\"\n")),
        (
            "lines",
            WHOLE_FILE.replace("note = \"n\"", "lines = \"3-4\"\nnote = \"n\""),
        ),
    ] {
        let dir = project(name, &toml);

        let (out, err) = checked(&dir);

        assert_eq!(out, "1 topic and 1 reference resolve\n", "{name}");
        assert_eq!(err, "", "{name}");
    }
}

#[test]
fn two_references_that_point_at_wholes_are_two_warnings() {
    let toml = format!("{WHOLE_FILE}\n[[references]]\nurl = \"{PAGE_URL}\"\nnote = \"n\"\n");
    let dir = project("two", &toml);

    let (out, err) = checked(&dir);

    assert_eq!(out, "1 topic and 2 references resolve, 2 warnings\n");
    assert_eq!(err.lines().count(), 2, "{err}");
}

#[test]
fn a_failure_still_lists_the_warnings() {
    let dir = project(
        "failure",
        &format!("{WHOLE_FILE}\n[[references]]\npath = \"gone.py\"\nnote = \"n\"\n"),
    );

    let output = refs(&dir, &["check"]);

    assert_eq!(output.status.code(), Some(1));
    let err = common::stderr(&output);
    assert!(err.contains("warning: "), "{err}");
    assert!(err.contains("gone.py"), "{err}");
}

#[test]
fn show_prints_the_quote_of_a_page_as_a_block_quote() {
    let dir = project(
        "show-quote",
        &page(PAGE_URL, "quote = \"First line.\\n\\nSecond line.\"\n"),
    );

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(
        text.contains(&format!(
            "## {PAGE_URL}\n\nn\n\n> First line.\n>\n> Second line.\n"
        )),
        "{text}"
    );
}

#[test]
fn export_carries_the_quote_of_a_page() {
    let dir = project(
        "export-quote",
        &page(PAGE_URL, "quote = \"The passage.\"\n"),
    );
    let cache = dir.join("cache").display().to_string();
    common::builtin::fill_cache(&dir.join("cache"));

    let output = refs(&dir, &["export", "--cache-dir", &cache]);

    let json: Value = serde_json::from_str(&ok(&output)).expect("valid JSON");
    let demo = json["topics"]
        .as_array()
        .expect("topics")
        .iter()
        .find(|topic| topic["id"] == "demo")
        .expect("the topic");
    assert_eq!(demo["references"][0]["quote"], "The passage.");
}

#[test]
fn a_quote_is_trimmed_and_followed_by_a_blank_line_before_the_next_reference() {
    let toml = format!(
        "{}\n[[references]]\nurl = \"{PAGE_URL}#next\"\nnote = \"after\"\n",
        page(PAGE_URL, "quote = \"\\n  Padded.  \\n\"\n")
    );
    let dir = project("quote-spacing", &toml);

    let text = ok(&refs(&dir, &["show", "demo"]));

    assert!(
        text.contains(&format!("n\n\n> Padded.\n\n## {PAGE_URL}#next\n")),
        "{text}"
    );
}
