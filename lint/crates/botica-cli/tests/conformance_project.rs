//! The conformance runner checks an extension rule of a catalog of a project with its fixtures, as
//! any other rule.

mod common;

use std::path::Path;

use botica_conformance::Runner;
use common::project_catalog::catalog;

fn runner(name: &str) -> Runner {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance-project")
        .join(format!("{name}-work"));
    Runner::new(env!("CARGO_BIN_EXE_botica"), scratch)
}

#[test]
fn an_extension_rule_of_a_project_catalog_conforms_to_its_fixtures() {
    let acme = catalog("conformance-project", "passes", "bad.py");

    let outcomes = runner("passes")
        .check_project_catalog(&acme)
        .expect("the catalog is checked");

    let names: Vec<&str> = outcomes.iter().map(|outcome| outcome.rule()).collect();
    assert_eq!(names, ["acme/by-sh"]);
    let reasons: Vec<String> = outcomes[0]
        .failures()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(outcomes[0].passed(), "{}", reasons.join("\n"));
}

#[test]
fn a_finding_on_a_valid_fixture_or_a_missing_one_on_an_invalid_fixture_fails_the_rule() {
    for (name, flags) in [
        ("flags-valid", "bad.py|ok.py"),
        ("flags-nothing", "other.py"),
    ] {
        let acme = catalog("conformance-project", name, flags);

        let outcomes = runner(name)
            .check_project_catalog(&acme)
            .expect("the catalog is checked");

        assert!(!outcomes[0].passed(), "{name}");
    }
}
