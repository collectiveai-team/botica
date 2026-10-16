//! The catalogs of a project, `.botica/rules/<catalog>/`, loaded on top of the built-in one.

mod common;

use botica_catalog::{Catalog, Kind};
use common::project::{
    acme, built_in, native, project_rule, rejected, AST_GREP_PYTHON, EXTENSION_COMMAND,
    EXTENSION_PYTHON,
};
use common::{family, file, rule_toml};

#[test]
fn a_project_catalog_adds_its_rules_after_the_built_in_ones() {
    let loaded = built_in()
        .with_projects(&acme())
        .unwrap_or_else(|error| panic!("{error}"));

    let ids: Vec<&str> = loaded.rules().iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(ids, ["no-utils", "acme/no-blobs"]);
}

#[test]
fn a_rule_of_a_project_is_known_by_its_prefixed_id_and_by_its_code() {
    let loaded = built_in().with_projects(&acme()).expect("loads");

    for key in ["acme/no-blobs", "ACMESTRUC001"] {
        let rule = loaded.resolve(key).unwrap_or_else(|| panic!("{key}"));
        assert_eq!(rule.id, "acme/no-blobs");
        assert_eq!(rule.catalog.as_deref(), Some("acme"));
    }
    assert_eq!(loaded.resolve("no-blobs"), None);
    assert_eq!(loaded.resolve("no-utils").expect("built in").catalog, None);
}

#[test]
fn the_families_of_a_project_catalog_have_the_code_of_the_catalog_in_front() {
    let loaded = built_in().with_projects(&acme()).expect("loads");

    let prefixes: Vec<String> = loaded.families().iter().map(|f| f.prefix()).collect();
    assert_eq!(prefixes, ["BTSTRUC", "ACMESTRUC"]);
    let catalogs: Vec<Option<&str>> = loaded
        .families()
        .iter()
        .map(|f| f.catalog.as_deref())
        .collect();
    assert_eq!(catalogs, [None, Some("acme")]);
}

#[test]
fn the_codes_of_the_catalogs_of_a_project_are_listed_without_the_built_in_one() {
    assert!(built_in().project_codes().is_empty());
    let loaded = built_in().with_projects(&acme()).expect("loads");

    assert_eq!(loaded.project_codes(), ["ACME"]);
    assert!(loaded.has_header("ACME") && loaded.has_header("BT"));
    assert!(!loaded.has_header("ZZ") && !built_in().has_header("ACME"));
}

#[test]
fn the_rules_of_a_project_catalog_follow_the_order_of_their_codes() {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "structure",
        "a-last",
        "ACMESTRUC003",
        "",
        AST_GREP_PYTHON,
    ));
    files.push(file(
        "acme/structure/a-last/python.yml",
        "id: acme/a-last\n",
    ));
    files.extend(project_rule(
        "acme",
        "structure",
        "z-middle",
        "ACMESTRUC002",
        "",
        AST_GREP_PYTHON,
    ));
    files.push(file(
        "acme/structure/z-middle/python.yml",
        "id: acme/z-middle\n",
    ));

    let loaded = built_in().with_projects(&files).expect("loads");

    let ids: Vec<&str> = loaded.rules().iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(
        ids,
        ["no-utils", "acme/no-blobs", "acme/z-middle", "acme/a-last"]
    );
}

#[test]
fn an_alias_of_a_project_rule_names_it() {
    let mut files = acme();
    files.retain(|f| !f.path.ends_with("rule.toml"));
    files.push(file(
        "acme/structure/no-blobs/rule.toml",
        &rule_toml(
            "acme/no-blobs",
            "ACMESTRUC001",
            "aliases = [\"acme/blobs\"]",
            AST_GREP_PYTHON,
        ),
    ));

    let loaded = built_in().with_projects(&files).expect("loads");

    assert_eq!(
        loaded.resolve("acme/blobs").map(|rule| rule.id.as_str()),
        Some("acme/no-blobs")
    );
}

#[test]
fn the_families_of_a_project_catalog_are_checked_against_each_other() {
    let mut files = acme();
    files.push(file(
        "acme/structures/family.toml",
        "code = \"STRUCT\"\ndescription = \"d\"\n",
    ));

    let message = rejected(&files);

    assert!(message.contains("selecting `ACMESTRUC`"), "{message}");
}

#[test]
fn a_folder_named_botica_is_a_family_like_any_other_in_a_project_catalog() {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "botica",
        "own",
        "ACMEBOT001",
        "",
        AST_GREP_PYTHON,
    ));
    files.push(file("acme/botica/own/python.yml", "id: acme/own\n"));

    let message = rejected(&files);

    assert!(message.contains("acme/botica/family.toml"), "{message}");
}

#[test]
fn an_ast_grep_rule_of_a_project_carries_its_yaml() {
    let loaded = built_in().with_projects(&acme()).expect("loads");

    let rule = loaded.resolve("acme/no-blobs").expect("the rule");
    assert_eq!(
        rule.implementations[0].source.as_deref(),
        Some("id: acme/no-blobs\n")
    );
}

#[test]
fn the_yaml_of_a_project_rule_declares_the_prefixed_id() {
    let mut files = acme();
    files.retain(|f| !f.path.ends_with("python.yml"));
    files.push(file("acme/structure/no-blobs/python.yml", "id: no-blobs\n"));

    let message = rejected(&files);

    assert!(message.contains("`id: acme/no-blobs`"), "{message}");
}

#[test]
fn a_project_with_no_catalog_changes_nothing() {
    let loaded = built_in().with_projects(&[]).expect("loads");

    assert_eq!(loaded.rules().len(), 1);
}

#[test]
fn the_id_of_a_project_rule_is_its_catalog_and_its_slug() {
    for id in ["no-blobs", "other/no-blobs", "acme/other"] {
        let mut files = acme();
        files.retain(|f| !f.path.ends_with("rule.toml"));
        files.push(file(
            "acme/structure/no-blobs/rule.toml",
            &rule_toml(id, "ACMESTRUC001", "", AST_GREP_PYTHON),
        ));

        let message = rejected(&files);

        assert!(
            message.contains("must equal `acme/no-blobs`"),
            "{id}: {message}"
        );
    }
}

#[test]
fn the_code_of_a_project_rule_is_the_code_of_its_catalog_and_its_family_and_three_digits() {
    for code in [
        "BTSTRUC001",
        "ACMESTRUC01",
        "ACMEARCH001",
        "ACMESTRUC0001",
        "ACME001",
    ] {
        let mut files = acme();
        files.retain(|f| !f.path.ends_with("rule.toml"));
        files.push(file(
            "acme/structure/no-blobs/rule.toml",
            &rule_toml("acme/no-blobs", code, "", AST_GREP_PYTHON),
        ));

        let message = rejected(&files);

        assert!(
            message.contains("must be `ACMESTRUC` followed by exactly 3 digits"),
            "{code}: {message}"
        );
    }
}

#[test]
fn a_project_catalog_takes_every_kind_but_native() {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "structure",
        "plain",
        "ACMESTRUC002",
        "",
        native(),
    ));

    let message = rejected(&files);

    assert!(message.contains("native"), "{message}");
    assert!(message.contains("acme/plain"), "{message}");
}

#[test]
fn a_project_catalog_takes_extension_rules() {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "structure",
        "by-script",
        "ACMESTRUC002",
        "",
        EXTENSION_PYTHON,
    ));

    let loaded = built_in().with_projects(&files).expect("loads");

    let rule = loaded.resolve("acme/by-script").expect("the rule");
    assert_eq!(rule.implementations[0].kind, Kind::Extension);
}

#[test]
fn the_command_runtime_is_for_a_project_catalog_and_never_for_the_built_in_one() {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "structure",
        "by-command",
        "ACMESTRUC002",
        "",
        EXTENSION_COMMAND,
    ));
    built_in()
        .with_projects(&files)
        .expect("a project may use it");

    let message = Catalog::from_files(
        &[
            vec![family("structure", "STRUC")],
            common::rule_with(
                "structure",
                "by-command",
                "BTSTRUC001",
                "",
                EXTENSION_COMMAND,
            ),
        ]
        .concat(),
    )
    .expect_err("the built-in catalog may not")
    .to_string();

    assert!(message.contains("command"), "{message}");
}
