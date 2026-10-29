//! The rules of a project catalog in the configuration: selectors, severity, overrides, replace.

mod common;

use botica_catalog::{Catalog, SourceFile};
use botica_config::{Config, Resolution, Use};
use common::{active, config, embedded, reason};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

fn rule(slug: &str, code: &str, enabled: bool, extra: &str) -> Vec<SourceFile> {
    let toml = format!(
        "id = \"acme/{slug}\"\ncode = \"{code}\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = {enabled}\n\
         default-severity = \"error\"\n{extra}\n[[implementations]]\nkind = \"ast-grep\"\n\
         languages = [\"python\"]\nfile = \"python.yml\"\n"
    );
    vec![
        file(&format!("acme/structure/{slug}/rule.toml"), &toml),
        file(&format!("acme/structure/{slug}/README.md"), README),
        file(
            &format!("acme/structure/{slug}/python.yml"),
            &format!("id: acme/{slug}\n"),
        ),
    ]
}

fn with_acme() -> Catalog {
    let files = [
        vec![
            file("acme/catalog.toml", "code = \"ACME\"\n"),
            file(
                "acme/structure/family.toml",
                "code = \"STRUC\"\ndescription = \"d\"\n",
            ),
        ],
        rule("no-blobs", "ACMESTRUC001", true, ""),
        rule(
            "better-utils",
            "ACMESTRUC002",
            false,
            "replaces = \"no-utils\"",
        ),
    ]
    .concat();
    embedded()
        .with_projects(&files)
        .unwrap_or_else(|error| panic!("{error}"))
}

fn resolve(text: &str) -> Resolution {
    let catalog = with_acme();
    Resolution::new(&config(text), &catalog).unwrap_or_else(|error| panic!("resolve:\n{error}"))
}

fn problems(text: &str) -> String {
    let parsed: Config = config(text);
    parsed
        .validate(&with_acme(), Use::TurnsOn)
        .expect_err("the configuration should be rejected")
        .to_string()
}

#[test]
fn a_rule_of_a_project_is_selected_by_id_code_family_or_code_prefix() {
    for selector in [
        "acme/no-blobs",
        "ACMESTRUC001",
        "ACMESTRUC",
        "ACMESTRUC0",
        "ACMESTRUC00",
    ] {
        let resolution = resolve(&format!("select = [\"{selector}\"]"));
        assert!(
            active(&resolution).contains(&"acme/no-blobs".to_owned()),
            "{selector}"
        );
    }
}

#[test]
fn a_selector_of_the_built_in_catalog_leaves_the_rules_of_a_project_alone() {
    let resolution = resolve("select = [\"BTSTRUC\"]");

    assert!(!active(&resolution).contains(&"acme/no-blobs".to_owned()));
    assert!(active(&resolution).contains(&"no-utils".to_owned()));
}

#[test]
fn a_selector_of_a_project_leaves_the_rules_of_the_built_in_catalog_alone() {
    let resolution = resolve("select = [\"ACMESTRUC\"]");

    assert_eq!(active(&resolution), ["acme/better-utils", "acme/no-blobs"]);
}

#[test]
fn a_selector_that_names_no_family_or_rule_of_a_project_is_an_error() {
    for bad in [
        "ACME",
        "ACMENOPE",
        "ACMESTRUC9",
        "ACMESTRUC0001",
        "ACMESTRUC002x",
        "acme/no-such-rule",
        "acme/",
        "/no-blobs",
        "acme/no/blobs",
        "Acme/no-blobs",
    ] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(message.contains("select[0]"), "{bad}: {message}");
    }
}

#[test]
fn a_text_that_is_not_a_code_or_a_rule_id_is_not_a_selector() {
    for bad in [
        "CES-71",
        "NoUtils",
        "acme/",
        "/no-blobs",
        "acme/no/blobs",
        "Acme/no-blobs",
        "",
    ] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(message.contains("is not a selector"), "{bad}: {message}");
    }
}

#[test]
fn digits_with_no_letters_name_a_rule_that_does_not_exist() {
    for bad in ["01", "001"] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(
            message.contains(&format!("unknown rule `{bad}`")),
            "{bad}: {message}"
        );
    }
}

#[test]
fn the_code_of_a_catalog_alone_selects_nothing_in_particular() {
    for alone in ["BT", "ACME"] {
        let message = problems(&format!("select = [\"{alone}\"]"));
        assert!(
            message.contains("alone selects nothing"),
            "{alone}: {message}"
        );
    }
}

#[test]
fn a_family_or_a_prefix_of_an_unknown_catalog_is_an_unknown_family() {
    for bad in ["NOPE", "NOPESTRUC", "NOPE0", "NOPESTRUC00"] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(message.contains("unknown family"), "{bad}: {message}");
    }
}

#[test]
fn the_selectors_of_the_built_in_catalog_keep_their_errors() {
    for bad in ["BT", "BTNOPE", "BTSTRUC9"] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(message.contains("select[0]"), "{bad}: {message}");
    }
}

#[test]
fn a_rule_of_a_project_is_active_by_default() {
    assert!(active(&resolve("")).contains(&"acme/no-blobs".to_owned()));
}

#[test]
fn a_rule_of_a_project_is_ignored_by_id_or_by_code() {
    for ignored in ["acme/no-blobs", "ACMESTRUC001", "ACMESTRUC"] {
        let resolution = resolve(&format!("ignore = [\"{ignored}\"]"));
        assert!(
            !active(&resolution).contains(&"acme/no-blobs".to_owned()),
            "{ignored}"
        );
    }
}

#[test]
fn a_rule_of_a_project_is_configured_under_its_quoted_id_or_its_code() {
    for key in ["\"acme/no-blobs\"", "ACMESTRUC001"] {
        let resolution = resolve(&format!("[rules.{key}]\nseverity = \"warning\"\n"));
        let entry = resolution.entry("acme/no-blobs").expect("the entry");
        assert_eq!(format!("{:?}", entry.severity), "Warning", "{key}");
    }
}

#[test]
fn an_override_ignores_or_changes_a_rule_of_a_project_by_path() {
    let resolution = resolve(
        "[[overrides]]\nfiles = [\"tests/**\"]\nignore = [\"ACMESTRUC\"]\n\
         [[overrides]]\nfiles = [\"scripts/**\"]\nseverity = { \"acme/no-blobs\" = \"info\" }\n",
    );

    let named = |path: &str| -> Vec<String> {
        resolution
            .rules_for(path, Some("python"))
            .iter()
            .map(|r| format!("{} {:?}", r.rule.id, r.severity))
            .collect()
    };
    assert!(!named("tests/a.py").iter().any(|r| r.starts_with("acme/")));
    assert!(named("scripts/a.py").contains(&"acme/no-blobs Info".to_owned()));
    assert!(named("src/a.py").contains(&"acme/no-blobs Error".to_owned()));
}

#[test]
fn a_project_rule_replaces_a_built_in_one_only_when_it_declares_it_and_the_config_confirms_it() {
    let resolution = resolve("[replace]\nno-utils = \"acme/better-utils\"\n");

    assert!(!active(&resolution).contains(&"no-utils".to_owned()));
    assert!(active(&resolution).contains(&"acme/better-utils".to_owned()));
    assert_eq!(
        format!("{:?}", reason(&resolution, "no-utils")),
        "ReplacedBy(\"acme/better-utils\")"
    );

    let unconfirmed = resolve("");
    assert!(active(&unconfirmed).contains(&"no-utils".to_owned()));
    assert!(!active(&unconfirmed).contains(&"acme/better-utils".to_owned()));
}

#[test]
fn a_replacement_the_rule_does_not_declare_is_an_error() {
    let message = problems_on_resolve("[replace]\nno-utils = \"acme/no-blobs\"\n");

    assert!(message.contains("does not declare"), "{message}");
}

fn problems_on_resolve(text: &str) -> String {
    let catalog = with_acme();
    Resolution::new(&config(text), &catalog)
        .expect_err("the replacement is rejected")
        .to_string()
}
