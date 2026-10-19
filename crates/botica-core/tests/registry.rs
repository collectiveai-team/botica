use std::collections::BTreeMap;

use botica_astgrep::AstGrepSet;
use botica_catalog::{Catalog, SourceFile};
use botica_core::{registry, Engine, Severity};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

fn native_rule(slug: &str, code: &str) -> Vec<SourceFile> {
    let toml = format!(
        r#"id = "{slug}"
code = "{code}"
summary = "A rule used by the tests"
status = "stable"
scope = "file"
needs = "bytes"
run-on = ["cli"]
default-enabled = true
default-severity = "error"

[[implementations]]
kind = "native"
languages = ["*"]
"#
    );
    vec![
        file(&format!("structure/{slug}/rule.toml"), &toml),
        file(&format!("structure/{slug}/README.md"), "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
        file(
            "structure/family.toml",
            "code = \"STRUC\"\ndescription = \"A family used by the tests\"\n",
        ),
    ]
}

#[test]
fn a_native_rule_of_the_catalog_without_an_implementation_is_rejected_by_name() {
    let catalog = Catalog::from_files(&native_rule("ghost-rule", "BTSTRUC900"))
        .expect("the catalog itself is valid");

    let error = registry::check_catalog(&catalog).expect_err("nothing implements `ghost-rule`");

    assert!(error.to_string().contains("ghost-rule"), "{error}");
}

#[test]
fn every_native_rule_of_the_shipped_catalog_has_an_implementation() {
    let catalog = Catalog::embedded().expect("the shipped catalog loads");

    registry::check_catalog(&catalog).expect("the catalog and the implementations agree");
}

#[test]
fn the_registry_builds_file_size_guard_with_the_thresholds_it_is_given() {
    let options = BTreeMap::from([
        ("warn".to_owned(), toml::Value::Integer(2)),
        ("max".to_owned(), toml::Value::Integer(3)),
    ]);
    let rule = registry::build("file-size-guard", &options, &registry::Context::default())
        .expect("file-size-guard is native");
    let engine = Engine::new(vec![rule], AstGrepSet::default());
    let lines = |count: usize| "x = 1\n".repeat(count);

    assert!(engine.lint_source("a.py", &lines(1)).is_empty());
    assert_eq!(
        engine.lint_source("a.py", &lines(2))[0].severity,
        Severity::Warning
    );
    assert_eq!(
        engine.lint_source("a.py", &lines(3))[0].severity,
        Severity::Error
    );
}
