//! The `references/` of this repository: its topics resolve, and each topic that a rule of the
//! built-in catalog or a standard cites exists. `botica refs check` leaves the citations of the
//! built-in catalog to this test, because they name the topics of this checkout.

mod common;

use std::path::{Path, PathBuf};

use botica_catalog::{read_references, Catalog, References, REFERENCES_DIR};
use common::{command, stderr, stdout};

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn topics() -> References {
    let files = read_references(&checkout().join(REFERENCES_DIR)).expect("read references/");
    References::from_files(&files).unwrap_or_else(|error| panic!("references/ loads:\n{error}"))
}

#[test]
fn the_references_of_this_repository_resolve() {
    let output = command(&checkout(), &["refs", "check"]);

    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
}

#[test]
fn every_topic_that_a_rule_of_the_built_in_catalog_cites_exists() {
    let topics = topics();
    let catalog = Catalog::embedded().expect("the built-in catalog loads");

    for rule in catalog.rules() {
        for cited in &rule.references {
            assert!(
                topics.topic(cited).is_some(),
                "rule {} cites the topic `{cited}`, which is not in references/",
                rule.id
            );
        }
    }
}

#[test]
fn the_standard_of_the_house_logger_shows_its_topic() {
    let output = command(&checkout(), &["refs", "show", "CES-74"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("structured-logging"), "{text}");
    assert!(text.contains("def get_logger"), "{text}");
}

#[test]
fn the_rules_of_logging_show_the_same_topic() {
    for rule in ["log-get-logger", "log-no-print"] {
        let output = command(&checkout(), &["refs", "show", rule]);

        assert!(output.status.success(), "{rule}: {}", stderr(&output));
        assert!(stdout(&output).contains("def _configure"), "{rule}");
    }
}

#[test]
fn the_topic_of_in_memory_adapters_shows_the_port_and_its_fake() {
    let output = command(&checkout(), &["refs", "show", "in-memory-adapters"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("class UserRepository(Protocol)"), "{text}");
    assert!(text.contains("class InMemoryUserRepository"), "{text}");
}

#[test]
fn the_standard_of_in_memory_adapters_shows_its_topic() {
    let output = command(&checkout(), &["refs", "show", "CES-64"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("class InMemoryUserRepository"));
}
