//! `botica refs` shows, checks and exports the files of the code of a topic.

mod common;

use std::path::PathBuf;

use common::refs::{add_topic, ok, refs};
use common::{project_in, write};
use serde_json::Value;

const LOGGER: &str =
    "import structlog\n\n\ndef get_logger(name):\n    return structlog.get_logger(name)\n";

const TOPIC: &str = "title = \"Logging\"\n\n[[references]]\nfile = \"code/core/logger.py\"\n\
                     note = \"the function that every module imports\"\n";

fn with_code(name: &str) -> PathBuf {
    let dir = project_in("refs", name, None);
    add_topic(&dir, "logging", TOPIC);
    write(&dir, "references/logging/code/core/logger.py", LOGGER);
    dir
}

#[test]
fn show_prints_the_file_of_the_topic_under_its_note() {
    let dir = with_code("code-show");

    let text = ok(&refs(&dir, &["show", "logging"]));

    assert!(
        text.contains(&format!(
            "## code/core/logger.py\n\nthe function that every module imports\n\n```\n{LOGGER}```\n"
        )),
        "{text}"
    );
}

#[test]
fn check_accepts_a_topic_whose_files_are_there_and_fails_when_one_is_missing() {
    let dir = with_code("code-check");
    assert!(ok(&refs(&dir, &["check"])).contains("1 reference"));

    std::fs::remove_file(dir.join("references/logging/code/core/logger.py")).expect("remove");
    let output = refs(&dir, &["check"]);

    assert_eq!(output.status.code(), Some(1));
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        errors.contains("`code/core/logger.py` is not a file of the topic"),
        "{errors}"
    );
}

#[test]
fn export_carries_the_file_and_its_text() {
    let dir = with_code("code-export");

    let json: Value = serde_json::from_str(&ok(&refs(&dir, &["export"]))).expect("valid JSON");

    let topic = json["topics"]
        .as_array()
        .expect("topics")
        .iter()
        .find(|topic| topic["id"] == "logging")
        .expect("the topic");
    let reference = &topic["references"][0];
    assert_eq!(reference["file"], "code/core/logger.py");
    assert_eq!(reference["path"], Value::Null);
    assert_eq!(reference["text"], LOGGER);
}
