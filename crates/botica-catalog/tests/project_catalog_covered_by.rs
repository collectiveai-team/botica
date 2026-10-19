//! A judgment rule of a project catalog names, in `covered-by`, the deterministic rules that check
//! part of it: those of its catalog by id and those of the built-in catalog by slug.

mod common;

use botica_catalog::{Catalog, SourceFile};
use common::project::{acme, built_in};
use common::{
    family, file, judgment, judgment_implementation, BOOLEAN_QUESTION, JUDGMENT_FIELDS,
    JUDGMENT_README,
};

/// `acme` and a judgment rule `acme/judge-blobs` of its family `testing` that covers `covered`.
fn acme_with_judgment(covered: &str) -> Vec<SourceFile> {
    let mut files = acme();
    files.push(family("acme/testing", "TEST"));
    files.push(file(
        "acme/testing/judge-blobs/rule.toml",
        &format!(
            "id = \"acme/judge-blobs\"\ncode = \"ACMETEST001\"\nsummary = \"s\"\n\
             status = \"preview\"\ndefault-enabled = false\ndefault-severity = \"warning\"\n\n\
             {JUDGMENT_FIELDS}covered-by = [{covered}]\n{BOOLEAN_QUESTION}"
        ),
    ));
    files.push(file("acme/testing/judge-blobs/README.md", JUDGMENT_README));
    files
}

fn covered_by(catalog: &Catalog) -> Vec<String> {
    catalog
        .resolve("acme/judge-blobs")
        .expect("the rule")
        .implementations[0]
        .covered_by
        .clone()
}

fn rejected(covered: &str) -> String {
    built_in()
        .with_projects(&acme_with_judgment(covered))
        .expect_err("the rule is rejected")
        .to_string()
}

#[test]
fn a_judgment_rule_of_a_project_is_covered_by_a_rule_of_the_built_in_catalog() {
    let catalog = built_in()
        .with_projects(&acme_with_judgment("\"no-utils\""))
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(covered_by(&catalog), ["no-utils"]);
}

#[test]
fn a_judgment_rule_of_a_project_is_covered_by_a_rule_of_its_own_catalog() {
    let catalog = built_in()
        .with_projects(&acme_with_judgment("\"acme/no-blobs\""))
        .unwrap_or_else(|error| panic!("{error}"));

    assert_eq!(covered_by(&catalog), ["acme/no-blobs"]);
}

#[test]
fn covered_by_names_a_rule_that_one_of_the_catalogs_holds() {
    let message = rejected("\"nope\"");

    assert!(
        message.contains("covered-by `nope` is not a rule of the catalog"),
        "{message}"
    );
}

#[test]
fn a_rule_of_a_project_is_named_by_its_id_and_not_by_its_slug() {
    let message = rejected("\"no-blobs\"");

    assert!(
        message.contains("covered-by `no-blobs` is not a rule of the catalog"),
        "{message}"
    );
}

#[test]
fn covered_by_does_not_name_a_judgment_rule_of_the_built_in_catalog() {
    let built_in = Catalog::from_files(&judgment("", &judgment_implementation()))
        .expect("the built-in catalog loads");

    let message = built_in
        .with_projects(&acme_with_judgment("\"test-through-interface\""))
        .expect_err("the rule is rejected")
        .to_string();

    assert!(
        message.contains("covered-by `test-through-interface` is a judgment rule"),
        "{message}"
    );
}
