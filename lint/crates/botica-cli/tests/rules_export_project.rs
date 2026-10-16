//! `botica rules export` describes the catalogs of the project, `.botica/rules/<name>`, after the
//! built-in one.

mod common;

use std::path::{Path, PathBuf};

use common::extension::{add_rule, project};
use common::{command, stderr};

fn exported(name: &str, format: &str) -> (PathBuf, PathBuf) {
    let dir = project(name, "");
    add_rule(&dir, "no-blobs", "ACMESTRUC001", "", "acme/no-blobs");
    let output = command(
        &dir,
        &["rules", "export", "--format", format, "--output", "out"],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let out = dir.join("out");
    (dir, out)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn the_rules_of_a_project_catalog_get_a_page_under_the_name_of_the_catalog() {
    let (_, out) = exported("markdown-page", "markdown");

    let page = read(&out.join("acme/acmestruc001-no-blobs.md"));

    let front_matter: Vec<&str> = page.lines().take_while(|line| !line.is_empty()).collect();
    for line in [
        "title: \"acme/no-blobs\"",
        "subtitle: \"ACMESTRUC001\"",
        "family: \"structure\"",
        "catalog: \"acme\"",
        "readme: \"acme/structure/no-blobs/README.md\"",
    ] {
        assert!(front_matter.contains(&line), "{line}\n{page}");
    }
    assert!(
        page.contains("\nFamily: structure · Catalog: acme · Languages: python\n"),
        "{page}"
    );
    assert!(page.contains("# acme/no-blobs (ACMESTRUC001)\n"), "{page}");
}

#[test]
fn the_index_has_a_section_for_each_catalog_of_the_project_after_those_of_the_built_in_one() {
    let (_, out) = exported("markdown-index", "markdown");

    let index = read(&out.join("index.md"));

    let section = "## acme\n\
                   \n\
                   Rules of the catalog `acme` of the project.\n\
                   \n\
                   | Code | Rule | Status | Added in | Default |\n\
                   |---|---|---|---|---|\n\
                   | `ACMESTRUC001` | [`acme/no-blobs`](acme/acmestruc001-no-blobs.md)<br>s | stable | unreleased | on |\n";
    assert!(index.contains(section), "{index}");
    assert!(
        index.contains("## structure\n"),
        "the built-in families stay: {index}"
    );
    assert!(
        index.find("## structure").expect("a built-in section")
            < index.find("## acme").expect("a project section"),
        "{index}"
    );
}

#[test]
fn the_json_names_the_catalog_of_a_rule_and_of_a_family() {
    let (_, out) = exported("json", "json");

    let document: serde_json::Value =
        serde_json::from_str(&read(&out.join("catalog.json"))).expect("JSON");

    let rules = document["rules"].as_array().expect("the rules");
    let rule = rules
        .iter()
        .find(|rule| rule["rule"] == "acme/no-blobs")
        .expect("the rule of the project");
    assert_eq!(rule["catalog"], "acme");
    assert_eq!(rule["readme"], "acme/structure/no-blobs/README.md");
    let built_in = rules
        .iter()
        .find(|rule| rule["rule"] == "no-utils")
        .expect("a built-in rule");
    assert!(built_in.get("catalog").is_none(), "{built_in}");
    let families = document["families"].as_array().expect("the families");
    let acme: Vec<_> = families
        .iter()
        .filter(|family| family["catalog"] == "acme")
        .collect();
    assert_eq!(acme.len(), 1, "{families:?}");
    assert_eq!(acme[0]["name"], "structure");
}

#[test]
fn a_project_whose_catalogs_do_not_load_cannot_be_exported() {
    let dir = project("broken", "");
    common::write(&dir, ".botica/rules/acme/structure/x/rule.toml", "nope\n");

    let output = command(
        &dir,
        &["rules", "export", "--format", "markdown", "--output", "out"],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.join("out").exists());
}
