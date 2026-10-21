//! What a catalog of a project must be to load.

mod common;

use botica_catalog::SourceFile;
use common::project::{acme, catalog_toml, project_rule, rejected, AST_GREP_PYTHON};
use common::{family, file, rule_toml};

#[test]
fn a_catalog_folder_needs_a_catalog_toml() {
    let files: Vec<SourceFile> = acme()
        .into_iter()
        .filter(|f| f.path != "acme/catalog.toml")
        .collect();

    let message = rejected(&files);

    assert!(message.contains("acme/catalog.toml"), "{message}");
}

#[test]
fn the_code_of_a_catalog_is_letters_that_botica_does_not_use() {
    for code in ["acme", "AC-ME", "AC1", "", "BT", "BTX", "B"] {
        let mut files = acme();
        files.retain(|f| f.path != "acme/catalog.toml");
        files.push(catalog_toml("acme", code));

        let message = rejected(&files);

        assert!(message.contains("acme/catalog.toml"), "`{code}`: {message}");
    }
}

#[test]
fn the_code_of_a_catalog_is_no_prefix_of_the_code_of_another() {
    let other = [
        vec![
            catalog_toml("acme-extra", "ACMEX"),
            family("acme-extra/arch", "ARCH"),
        ],
        project_rule(
            "acme-extra",
            "arch",
            "one",
            "ACMEXARCH001",
            "",
            AST_GREP_PYTHON,
        ),
        vec![file(
            "acme-extra/arch/one/python.yml",
            "id: acme-extra/one\n",
        )],
    ]
    .concat();

    let message = rejected(&[acme(), other].concat());

    assert!(message.contains("`ACME`"), "{message}");
    assert!(message.contains("`ACMEX`"), "{message}");
}

#[test]
fn two_catalogs_do_not_share_a_code() {
    let other = [
        vec![
            catalog_toml("second", "ACME"),
            family("second/arch", "ARCH"),
        ],
        project_rule("second", "arch", "one", "ACMEARCH001", "", AST_GREP_PYTHON),
        vec![file("second/arch/one/python.yml", "id: second/one\n")],
    ]
    .concat();

    let message = rejected(&[acme(), other].concat());

    assert!(message.contains("`ACME`"), "{message}");
    assert_eq!(
        message.matches("is already the code").count(),
        1,
        "{message}"
    );
}

#[test]
fn the_name_of_a_catalog_is_kebab_case() {
    let files: Vec<SourceFile> = acme()
        .into_iter()
        .map(|f| SourceFile {
            path: f.path.replacen("acme/", "Acme/", 1),
            text: f.text,
        })
        .collect();

    let message = rejected(&files);

    assert!(message.contains("kebab-case"), "{message}");
}

#[test]
fn a_catalog_does_not_take_the_name_of_a_family_of_the_built_in_catalog() {
    let files = [
        vec![
            catalog_toml("structure", "ACME"),
            family("structure/arch", "ARCH"),
        ],
        project_rule(
            "structure",
            "arch",
            "one",
            "ACMEARCH001",
            "",
            AST_GREP_PYTHON,
        ),
        vec![file("structure/arch/one/python.yml", "id: structure/one\n")],
    ]
    .concat();

    let message = rejected(&files);

    assert!(message.contains("`structure`"), "{message}");
    assert!(
        message.contains("family of the built-in catalog"),
        "{message}"
    );
}

#[test]
fn an_alias_of_a_project_rule_cannot_be_the_id_or_the_code_of_a_built_in_one() {
    for alias in ["no-utils", "BTSTRUC001"] {
        let mut files = acme();
        files.retain(|f| !f.path.ends_with("rule.toml"));
        files.push(file(
            "acme/structure/no-blobs/rule.toml",
            &rule_toml(
                "acme/no-blobs",
                "ACMESTRUC001",
                &format!("aliases = [\"{alias}\"]"),
                AST_GREP_PYTHON,
            ),
        ));

        let message = rejected(&files);

        assert!(message.contains("collides"), "{alias}: {message}");
    }
}

#[test]
fn a_family_of_a_project_catalog_needs_its_family_toml() {
    let files: Vec<SourceFile> = acme()
        .into_iter()
        .filter(|f| f.path != "acme/structure/family.toml")
        .collect();

    let message = rejected(&files);

    assert!(message.contains("acme/structure/family.toml"), "{message}");
}

#[test]
fn a_file_that_a_project_catalog_does_not_expect_is_rejected() {
    let mut files = acme();
    files.push(file("acme/notes.txt", "x"));

    let message = rejected(&files);

    assert!(message.contains("acme/notes.txt"), "{message}");
}

#[test]
fn the_history_belongs_to_the_built_in_catalog() {
    let mut files = acme();
    files.push(file("acme/history.toml", ""));

    let message = rejected(&files);

    assert!(message.contains("acme/history.toml"), "{message}");
}

#[test]
fn a_file_with_no_catalog_folder_is_rejected() {
    let mut files = acme();
    files.push(file("catalog.toml", "code = \"ZZ\"\n"));

    let message = rejected(&files);

    assert!(message.contains("catalog.toml"), "{message}");
    assert!(message.contains("unexpected file"), "{message}");
}

#[test]
fn the_issues_of_a_project_catalog_name_its_files_from_the_rules_folder() {
    let mut files = acme();
    files.retain(|f| !f.path.ends_with("README.md"));

    let message = rejected(&files);

    assert!(
        message.contains("acme/structure/no-blobs/README.md"),
        "{message}"
    );
}
