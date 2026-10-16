//! A family of a project catalog with the name of a built-in family says, with `extends` in its
//! `family.toml`, that it extends that family on purpose.

mod common;

use botica_catalog::{Catalog, SourceFile};
use common::project::{acme, built_in, catalog_toml, project_rule, rejected, AST_GREP_PYTHON};
use common::{file, issues};

fn family_toml(dir: &str, code: &str, extra: &str) -> SourceFile {
    file(
        &format!("{dir}/family.toml"),
        &format!("code = \"{code}\"\ndescription = \"d\"\n{extra}"),
    )
}

/// `acme` with the family `structure` replaced by one whose `family.toml` has `extra`.
fn acme_structure(extra: &str) -> Vec<SourceFile> {
    let mut files = acme();
    files.retain(|f| f.path != "acme/structure/family.toml");
    files.push(family_toml("acme/structure", "STRUC", extra));
    files
}

/// `acme` with the family `blobs`, which no built-in family is called, whose `family.toml` has
/// `extra`.
fn acme_blobs(extra: &str) -> Vec<SourceFile> {
    [
        vec![
            catalog_toml("acme", "ACME"),
            family_toml("acme/blobs", "BLOB", extra),
        ],
        project_rule(
            "acme",
            "blobs",
            "no-blobs",
            "ACMEBLOB001",
            "",
            AST_GREP_PYTHON,
        ),
        vec![file(
            "acme/blobs/no-blobs/python.yml",
            "id: acme/no-blobs\n",
        )],
    ]
    .concat()
}

fn loaded(files: &[SourceFile]) -> Catalog {
    built_in()
        .with_projects(files)
        .unwrap_or_else(|error| panic!("the catalogs load:\n{error}"))
}

#[test]
fn a_project_family_with_the_name_of_a_built_in_one_is_warned_about() {
    let catalog = loaded(&acme());

    let warnings = catalog.warnings();

    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("`structure`"), "{warnings:?}");
    assert!(warnings[0].contains("`acme`"), "{warnings:?}");
    assert!(
        warnings[0].contains("extends = \"structure\""),
        "{warnings:?}"
    );
}

#[test]
fn a_family_that_says_it_extends_the_built_in_one_is_not_warned_about() {
    let catalog = loaded(&acme_structure("extends = \"structure\"\n"));

    assert!(catalog.warnings().is_empty(), "{:?}", catalog.warnings());
    let family = catalog
        .families()
        .iter()
        .find(|family| family.catalog.as_deref() == Some("acme"))
        .expect("the family of the project");
    assert_eq!(family.extends.as_deref(), Some("structure"));
    let rule = catalog.resolve("acme/no-blobs").expect("the rule");
    assert_eq!(rule.extends.as_deref(), Some("structure"));
}

#[test]
fn a_project_family_with_another_name_needs_no_mark_and_may_extend_one() {
    assert!(loaded(&acme_blobs("")).warnings().is_empty());

    let catalog = loaded(&acme_blobs("extends = \"structure\"\n"));

    assert!(catalog.warnings().is_empty());
    let rule = catalog.resolve("acme/no-blobs").expect("the rule");
    assert_eq!(rule.family.as_deref(), Some("blobs"));
    assert_eq!(rule.extends.as_deref(), Some("structure"));
}

#[test]
fn a_built_in_family_without_a_mark_is_not_warned_about() {
    assert!(built_in().warnings().is_empty());
}

#[test]
fn extends_names_a_family_of_the_built_in_catalog() {
    let message = rejected(&acme_blobs("extends = \"nope\"\n"));

    assert!(message.contains("acme/blobs/family.toml"), "{message}");
    assert!(message.contains("`nope`"), "{message}");
    assert!(message.contains("extends"), "{message}");
}

#[test]
fn a_family_cannot_extend_the_family_of_another_project_catalog() {
    let second = [
        vec![
            catalog_toml("second", "SECO"),
            family_toml("second/more", "MORE", "extends = \"blobs\"\n"),
        ],
        project_rule(
            "second",
            "more",
            "other",
            "SECOMORE001",
            "",
            AST_GREP_PYTHON,
        ),
        vec![file("second/more/other/python.yml", "id: second/other\n")],
    ]
    .concat();

    let message = rejected(&[acme_blobs(""), second].concat());

    assert!(message.contains("second/more/family.toml"), "{message}");
    assert!(message.contains("`blobs`"), "{message}");
}

#[test]
fn the_built_in_catalog_has_no_family_to_extend() {
    let message = issues(vec![vec![file(
        "structure/family.toml",
        "code = \"STRUC\"\ndescription = \"d\"\nextends = \"structure\"\n",
    )]]);

    assert!(message.contains("structure/family.toml"), "{message}");
    assert!(message.contains("extends"), "{message}");
}

#[test]
fn a_family_of_the_built_in_catalog_has_no_extends() {
    let catalog = built_in();

    assert!(catalog
        .families()
        .iter()
        .all(|family| family.extends.is_none()));
    assert!(catalog.rules().iter().all(|rule| rule.extends.is_none()));
}
