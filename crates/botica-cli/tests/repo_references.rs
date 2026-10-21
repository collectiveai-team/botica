//! The topics of this repository: the rules and the standards cite topics that exist, and the
//! topics that the binary carries show what the standards ask for. The references of those topics
//! are fetched by `botica refs check --all`, which the job `test` of `ci.yml` runs.

mod common;

use std::path::{Path, PathBuf};

use botica_catalog::{Catalog, References};
use common::builtin::fill_cache;
use common::{command, stderr, stdout};

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cache(name: &str) -> PathBuf {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("repo-references")
        .join(name);
    let _ = std::fs::remove_dir_all(&cache);
    fill_cache(&cache);
    cache
}

/// `botica refs <args>` in this checkout, with a cache that holds the files of the built-in topics.
fn refs_here(name: &str, args: &[&str]) -> String {
    let cache = cache(name);
    let cache = cache.display().to_string();
    let mut all = vec!["refs"];
    all.extend_from_slice(args);
    all.extend(["--cache-dir", &cache]);
    let output = command(&checkout(), &all);
    assert!(
        output.status.success(),
        "{name}: {}{}",
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

#[test]
fn the_citations_of_this_repository_resolve() {
    refs_here("check", &["check"]);
}

#[test]
fn every_topic_that_a_rule_of_the_built_in_catalog_cites_is_built_in() {
    let topics = References::embedded().expect("the embedded topics load");
    let catalog = Catalog::embedded().expect("the built-in catalog loads");

    for rule in catalog.rules() {
        for cited in &rule.references {
            assert!(
                topics.topic(cited).is_some(),
                "rule {} cites the topic `{cited}`, which catalog/references/ does not have",
                rule.id
            );
        }
    }
}

#[test]
fn the_standard_of_the_house_logger_shows_its_topic() {
    let text = refs_here("ces-74", &["show", "CES-74"]);

    assert!(text.contains("# structured-logging: "), "{text}");
    assert!(text.contains("def get_logger"), "{text}");
}

#[test]
fn the_rules_of_logging_show_the_same_topic() {
    for rule in ["log-get-logger", "log-no-print"] {
        let text = refs_here(rule, &["show", rule]);

        assert!(text.contains("def _configure"), "{rule}: {text}");
    }
}

#[test]
fn the_standard_of_in_memory_adapters_shows_its_topic() {
    let text = refs_here("ces-64", &["show", "CES-64"]);

    assert!(text.contains("class InMemoryUserRepository"), "{text}");
}
