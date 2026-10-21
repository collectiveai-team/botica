//! A topic carries its code under `code/`, and a `file` reference shows a file of it.

mod common;

use botica_catalog::{References, SourceFile};
use common::file;
use common::topics::{load, rejected, topic};

const LOGGER: &str =
    "import structlog\n\n\ndef get_logger(name):\n    return structlog.get_logger(name)\n";

const WITH_FILE: &str = r#"title = "Structured logging"

[[references]]
file = "code/core/logger.py"
lines = "4-5"
anchor = "def get_logger"
note = "the function that every module imports"
"#;

fn with_code(toml: &str, code: &[(&str, &str)]) -> Vec<SourceFile> {
    let mut files = topic("logging", toml);
    files.extend(
        code.iter()
            .map(|(path, text)| file(&format!("logging/{path}"), text)),
    );
    files
}

#[test]
fn a_file_reference_shows_a_file_of_the_code_of_its_topic() {
    let loaded = load(vec![with_code(
        WITH_FILE,
        &[("code/core/logger.py", LOGGER)],
    )])
    .expect("loads");

    let found = loaded.topic("logging").expect("the topic");
    let reference = &found.references[0];
    assert!(reference.is_topic_file());
    assert!(!reference.is_external());
    assert_eq!(found.file("code/core/logger.py"), Some(LOGGER));
    assert_eq!(
        reference.excerpt(LOGGER),
        "def get_logger(name):\n    return structlog.get_logger(name)\n"
    );
    assert_eq!(reference.suggestion(), None);
}

#[test]
fn a_built_in_topic_may_show_its_own_code() {
    References::from_builtin_files(&with_code(WITH_FILE, &[("code/core/logger.py", LOGGER)]))
        .expect("loads");
}

#[test]
fn a_file_that_the_topic_does_not_carry_is_an_error() {
    let message = rejected(vec![with_code(WITH_FILE, &[])]);

    assert!(message.contains("logging/topic.toml"), "{message}");
    assert!(
        message.contains("`code/core/logger.py` is not a file of the topic"),
        "{message}"
    );
}

#[test]
fn the_lines_and_the_anchor_are_checked_when_the_topic_loads() {
    let past = WITH_FILE.replace("lines = \"4-5\"", "lines = \"4-9\"");
    let message = rejected(vec![with_code(&past, &[("code/core/logger.py", LOGGER)])]);
    assert!(message.contains("past the end of the file"), "{message}");

    let moved = WITH_FILE.replace("lines = \"4-5\"", "lines = \"1-2\"");
    let message = rejected(vec![with_code(&moved, &[("code/core/logger.py", LOGGER)])]);
    assert!(
        message.contains("the anchor `def get_logger` is not in lines 1-2"),
        "{message}"
    );
}

#[test]
fn a_file_reference_names_a_path_under_code() {
    for bad in ["core/logger.py", "code/../README.md", "code/", "/code/a.py"] {
        let toml = WITH_FILE.replace("code/core/logger.py", bad);
        let message = rejected(vec![with_code(&toml, &[("code/core/logger.py", LOGGER)])]);
        assert!(message.contains("under `code/`"), "{bad}: {message}");
    }
}

#[test]
fn a_file_reference_takes_no_repository_license_or_quote() {
    for (field, line) in [
        ("repo", "repo = \"github.com/acme/app\"\n"),
        ("license", "license = \"MIT\"\n"),
        ("quote", "quote = \"a passage\"\n"),
    ] {
        let toml = format!("{WITH_FILE}{line}");
        let message = rejected(vec![with_code(&toml, &[("code/core/logger.py", LOGGER)])]);
        assert!(
            message.contains(&format!("`{field}` does not apply to a `file` reference")),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_file_outside_code_is_not_part_of_a_topic() {
    let message = rejected(vec![with_code(WITH_FILE, &[("tests/test_logger.py", "")])]);

    assert!(message.contains("unexpected file"), "{message}");
}
