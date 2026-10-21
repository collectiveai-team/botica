//! `botica rules conform <catalog>` runs the fixtures of each rule of a catalog.

mod common;

use std::path::Path;

use common::project_catalog::catalog;
use common::{command, stderr, stdout};

fn conform(acme: &Path) -> std::process::Output {
    let work = acme.parent().expect("the catalog has a folder");
    command(
        work,
        &["rules", "conform", acme.to_str().expect("a UTF-8 path")],
    )
}

#[test]
fn a_catalog_of_a_project_whose_rules_conform_exits_zero_and_names_each_rule() {
    let acme = catalog("rules-conform", "passes", "bad.py");

    let output = conform(&acme);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("ok   acme/by-sh"), "{text}");
    assert!(text.contains("1 of 1 rules conform"), "{text}");
}

#[test]
fn the_projects_of_the_fixtures_ignore_themselves() {
    let acme = catalog("rules-conform", "ignored", "bad.py");

    let output = conform(&acme);

    assert!(output.status.success(), "{}", stderr(&output));
    let scratch = acme.parent().expect("a folder").join(".botica/conformance");
    assert_eq!(
        std::fs::read_to_string(scratch.join(".gitignore")).expect("ignored"),
        "*\n"
    );
}

#[test]
fn the_built_in_catalog_conforms_by_the_slug_of_each_rule() {
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/rules");
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rules-conform-built-in");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("a working directory");

    let output = command(
        &work,
        &["rules", "conform", rules.to_str().expect("a UTF-8 path")],
    );

    assert!(output.status.success(), "{}", stdout(&output));
    let text = stdout(&output);
    assert!(text.contains("ok   no-utils\n"), "{text}");
    assert!(text.contains(" rules conform"), "{text}");
}

#[test]
fn a_rule_that_does_not_conform_exits_one_and_says_why() {
    let acme = catalog("rules-conform", "flags-valid", "bad.py|ok.py");

    let output = conform(&acme);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("FAIL acme/by-sh"), "{text}");
    assert!(text.contains("a valid fixture raised"), "{text}");
    assert!(text.contains("0 of 1 rules conform"), "{text}");
}
