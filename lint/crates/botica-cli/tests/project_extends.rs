//! A family of a project with the name of a built-in family: a warning, unless its `family.toml`
//! says `extends`, and in the export its rules sit under the built-in family.

mod common;

use std::path::{Path, PathBuf};

use common::extension::{add_rule, project};
use common::{command, stderr, write};

const FAMILY: &str = ".botica/rules/acme/structure/family.toml";

fn with_family(name: &str, family_toml: &str) -> PathBuf {
    let dir = project(name, "");
    write(&dir, FAMILY, family_toml);
    add_rule(&dir, "no-blobs", "ACMESTRUC001", "", "acme/no-blobs");
    dir
}

const UNMARKED: &str = "code = \"STRUC\"\ndescription = \"d\"\n";
const MARKED: &str = "code = \"STRUC\"\ndescription = \"d\"\nextends = \"structure\"\n";

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_family_with_the_name_of_a_built_in_one_is_a_warning_on_stderr_and_does_not_fail_the_run() {
    let dir = with_family("warns", UNMARKED);

    let output = command(&dir, &["rules"]);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stderr(&output);
    assert!(text.starts_with("botica: warning: "), "{text}");
    assert!(text.contains("extends = \"structure\""), "{text}");
}

#[test]
fn a_family_that_extends_the_built_in_one_says_nothing() {
    let dir = with_family("marked", MARKED);

    let output = command(&dir, &["rules"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

#[test]
fn a_family_that_extends_a_family_that_is_not_built_in_fails_the_run() {
    let dir = with_family(
        "unknown",
        "code = \"STRUC\"\ndescription = \"d\"\nextends = \"nope\"\n",
    );

    let output = command(&dir, &["rules"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("`nope`"), "{}", stderr(&output));
}

#[test]
fn the_export_puts_the_rules_of_an_extending_family_under_the_built_in_family_after_its_rules() {
    let dir = with_family("export", MARKED);
    let output = command(
        &dir,
        &["rules", "export", "--format", "markdown", "--output", "out"],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let out = dir.join("out");

    assert!(out.join("structure/acmestruc001-no-blobs.md").is_file());
    assert!(out
        .join("structure/btstruc001-file-size-guard.md")
        .is_file());
    assert!(!out.join("acme").exists());
    let index = read(&out.join("index.md"));
    assert!(!index.contains("## acme"), "{index}");
    assert_eq!(index.matches("## structure\n").count(), 1, "{index}");
    let built_in = index.find("| `BTSTRUC001`").expect("a built-in row");
    let project = index
        .find("| `ACMESTRUC001`")
        .expect("a row of the project");
    assert!(built_in < project, "{index}");
    assert!(
        index.contains("(structure/acmestruc001-no-blobs.md)"),
        "{index}"
    );
    let page = read(&out.join("structure/acmestruc001-no-blobs.md"));
    assert!(page.contains("\ncatalog: \"acme\"\n"), "{page}");
    assert!(page.contains("\nfamily: \"structure\"\n"), "{page}");
}

#[test]
fn the_json_export_says_which_family_extends_which() {
    let dir = with_family("json", MARKED);
    let output = command(
        &dir,
        &["rules", "export", "--format", "json", "--output", "out"],
    );
    assert!(output.status.success(), "{}", stderr(&output));

    let document: serde_json::Value =
        serde_json::from_str(&read(&dir.join("out/catalog.json"))).expect("JSON");

    let families = document["families"].as_array().expect("the families");
    let acme = families
        .iter()
        .find(|family| family["catalog"] == "acme")
        .expect("the family of the project");
    assert_eq!(acme["extends"], "structure");
    let built_in = families
        .iter()
        .find(|family| family["name"] == "structure" && family.get("catalog").is_none())
        .expect("the built-in family");
    assert!(built_in.get("extends").is_none(), "{built_in}");
}

#[test]
fn without_the_mark_the_export_keeps_the_rules_of_the_project_in_the_folder_of_their_catalog() {
    let dir = with_family("export-unmarked", UNMARKED);
    let output = command(
        &dir,
        &["rules", "export", "--format", "markdown", "--output", "out"],
    );
    assert!(output.status.success(), "{}", stderr(&output));

    assert!(dir.join("out/acme/acmestruc001-no-blobs.md").is_file());
    assert!(!dir.join("out/structure/acmestruc001-no-blobs.md").exists());
}
