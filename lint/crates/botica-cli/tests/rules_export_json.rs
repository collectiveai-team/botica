use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

fn run(name: &str, args: &[&str]) -> (PathBuf, Output) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-export-json")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the working directory");
    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(&dir)
        .output()
        .expect("run botica");
    (dir, output)
}

fn catalog(name: &str) -> Value {
    let (dir, output) = run(
        name,
        &["rules", "export", "--format", "json", "--output", "out"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(dir.join("out/catalog.json")).expect("out/catalog.json");
    serde_json::from_str(&text).expect("a JSON document")
}

fn rule<'a>(document: &'a Value, slug: &str) -> &'a Value {
    document["rules"]
        .as_array()
        .expect("the rules")
        .iter()
        .find(|rule| rule["rule"] == slug)
        .unwrap_or_else(|| panic!("no rule {slug}"))
}

#[test]
fn writes_one_catalog_json_with_every_rule_of_the_catalog_in_the_order_of_their_codes() {
    let document = catalog("every-rule");

    let (_, listing) = run("every-rule-listing", &["rules", "--all", "--json"]);
    let listed: Value = serde_json::from_slice(&listing.stdout).expect("JSON");
    let mut expected: Vec<&str> = listed["rules"]
        .as_array()
        .expect("the rules")
        .iter()
        .map(|rule| rule["code"].as_str().expect("a code"))
        .collect();
    expected.sort_unstable();
    let codes: Vec<&str> = document["rules"]
        .as_array()
        .expect("the rules")
        .iter()
        .map(|rule| rule["code"].as_str().expect("a code"))
        .collect();

    assert_eq!(document["schema_version"], 1);
    assert!(codes.len() > 20, "{} rules", codes.len());
    assert_eq!(codes, expected);
    assert_eq!(rule(&document, "io-error")["code"], json!("BT001"));
}

fn without_sections(rule: &Value) -> Value {
    let mut fields = rule.clone();
    fields
        .as_object_mut()
        .expect("a rule is an object")
        .remove("sections");
    fields
}

#[test]
fn a_rule_carries_every_field_of_its_rule_toml_its_releases_and_the_path_of_its_readme() {
    let document = catalog("fields");

    assert_eq!(
        without_sections(rule(&document, "api-schemas-extra-forbid")),
        json!({
            "rule": "api-schemas-extra-forbid",
            "code": "BTFAST001",
            "family": "fastapi",
            "standard": "CES-4",
            "summary": "Request and response schemas reject unknown fields",
            "status": "stable",
            "added": "0.8.0",
            "since": "0.8.0",
            "aliases": [],
            "replaces": null,
            "tags": ["typing"],
            "files": [],
            "ignores": [],
            "prefilter": null,
            "scope": "file",
            "needs": "syntax",
            "run_on": ["cli", "save", "type"],
            "default_enabled": true,
            "default_severity": "warning",
            "dependencies": [],
            "options": {},
            "implementations": [{
                "kind": "ast-grep",
                "runtime": null,
                "languages": ["python"],
                "status": "stable",
                "file": "python.yml",
                "when": null,
                "question": null,
                "covered_by": [],
                "why_not_deterministic": null
            }],
            "readme": "fastapi/api-schemas-extra-forbid/README.md"
        })
    );
}

#[test]
fn a_rule_carries_its_options_with_their_type_default_and_minimum() {
    let document = catalog("options");

    let guard = rule(&document, "file-size-guard");
    assert_eq!(
        guard["options"],
        json!({
            "max": {"type": "int", "default": 700, "min": 1},
            "warn": {"type": "int", "default": 400, "min": 1}
        })
    );
    assert_eq!(guard["implementations"][0]["kind"], "native");
    assert_eq!(guard["needs"], "bytes");
    assert_eq!(guard["default_severity"], "error");
    assert_eq!(
        rule(&document, "no-dict-literal-return")["prefilter"],
        json!("(?-u)\\breturn[\\s\\\\]*\\{")
    );
}

#[test]
fn a_rule_of_botica_itself_has_no_family_and_its_readme_is_under_botica() {
    let document = catalog("own");

    let io_error = rule(&document, "io-error");
    assert_eq!(io_error["family"], Value::Null);
    assert_eq!(io_error["standard"], Value::Null);
    assert_eq!(io_error["readme"], "botica/io-error/README.md");
}

#[test]
fn a_rule_carries_the_sections_of_its_readme_in_their_order() {
    let document = catalog("sections");

    let sections = rule(&document, "log-no-print")["sections"]
        .as_array()
        .expect("the sections")
        .clone();
    let titles: Vec<&str> = sections
        .iter()
        .map(|section| section["title"].as_str().expect("a title"))
        .collect();
    assert_eq!(
        titles,
        [
            "Directive",
            "Why",
            "Exceptions",
            "Do",
            "Don't",
            "Suppression"
        ]
    );
    assert_eq!(
        sections[0]["text"],
        "Importable library code logs through a logger. A `print()` in Python or a \
         `console.log()` in\nTypeScript is a violation. A warning by default."
    );
}

#[test]
fn lists_the_families_of_the_catalog_by_name_with_their_code_and_description() {
    let document = catalog("families");

    let families = document["families"].as_array().expect("the families");
    let names: Vec<&str> = families
        .iter()
        .map(|family| family["name"].as_str().expect("a name"))
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
    assert!(families.contains(&json!({
        "name": "fastapi",
        "code": "FAST",
        "description": "Rules for FastAPI services."
    })));
}
