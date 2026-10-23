use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use botica_catalog::{Catalog, Kind, OWN_RULES_DIR};

fn rules_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/rules")
}

fn embedded() -> Catalog {
    Catalog::embedded().unwrap_or_else(|error| panic!("the built-in catalog is invalid:\n{error}"))
}

/// The files under `dir`, at any depth: a rule scoped to a placement keeps its fixtures in it.
fn files_in(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| {
                    let path = e.path();
                    if path.is_file() {
                        1
                    } else {
                        files_in(&path)
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

#[test]
fn the_built_in_catalog_loads() {
    let catalog = embedded();
    for slug in [
        "file-size-guard",
        "no-utils",
        "io-error",
        "unused-suppression",
        "invalid-suppression",
    ] {
        assert!(catalog.resolve(slug).is_some(), "{slug} is missing");
    }
    assert_eq!(
        catalog.resolve("BTSTRUC001").map(|r| r.id.as_str()),
        Some("file-size-guard")
    );
    assert_eq!(
        catalog.resolve("BT001").map(|r| r.id.as_str()),
        Some("io-error")
    );
    assert_eq!(
        catalog.resolve("BT003").map(|r| r.id.as_str()),
        Some("invalid-suppression")
    );
}

#[test]
fn the_families_are_the_ones_the_spec_lists() {
    let catalog = embedded();
    let mut codes: Vec<&str> = catalog.families().iter().map(|f| f.code.as_str()).collect();
    codes.sort_unstable();
    let mut expected = [
        "STRUC", "ARCH", "CONF", "DB", "CLI", "LOG", "DM", "FAST", "PROC", "SEC", "TEST", "TOOL",
        "WRIT", "DOCS",
    ];
    expected.sort_unstable();
    assert_eq!(codes, expected);
}

#[test]
fn the_linters_own_rules_have_no_family() {
    let catalog = embedded();
    for rule in catalog.rules() {
        let own = rule.family.is_none();
        assert_eq!(own, rule.code.len() == 5, "{}: {}", rule.id, rule.code);
    }
}

#[test]
fn every_rule_that_botica_or_a_judge_evaluates_has_valid_and_invalid_fixtures() {
    let catalog = embedded();
    for rule in catalog
        .rules()
        .iter()
        .filter(|rule| !rule.is_external_only())
    {
        let family = rule.family.as_deref().unwrap_or(OWN_RULES_DIR);
        let fixtures = rules_dir().join(family).join(&rule.id).join("fixtures");
        for kind in ["valid", "invalid"] {
            assert!(
                files_in(&fixtures.join(kind)) > 0,
                "{}: fixtures/{kind} has no files",
                rule.id
            );
        }
    }
}

#[test]
fn the_message_of_an_ast_grep_rule_carries_no_standard() {
    let catalog = embedded();
    for rule in catalog.rules() {
        let Some(standard) = &rule.standard else {
            continue;
        };
        for yaml in rule
            .implementations
            .iter()
            .filter(|implementation| implementation.kind == Kind::AstGrep)
            .filter_map(|implementation| implementation.source.as_deref())
        {
            let message = yaml
                .lines()
                .find(|line| line.starts_with("message:"))
                .unwrap_or_default();
            assert!(
                !message.contains(standard.as_str()),
                "{}: the message names `{standard}`, which the output takes from rule.toml",
                rule.id
            );
        }
    }
}

#[test]
fn no_ruff_prefix_starts_with_the_botica_header() {
    let output = match Command::new("ruff").arg("linter").output() {
        Ok(output) if output.status.success() => output,
        _ if std::env::var_os("BOTICA_REQUIRE_RUFF").is_some() => {
            panic!("BOTICA_REQUIRE_RUFF is set but `ruff linter` failed")
        }
        _ => {
            eprintln!("skipped: `ruff` is not installed");
            return;
        }
    };
    let listing = String::from_utf8(output.stdout).expect("ruff prints UTF-8");
    let prefixes: Vec<&str> = listing
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert!(
        prefixes.len() > 50,
        "unexpected `ruff linter` output: {listing}"
    );
    let clashes: Vec<&&str> = prefixes.iter().filter(|p| p.starts_with("BT")).collect();
    assert!(
        clashes.is_empty(),
        "ruff now owns prefixes that start with BT: {clashes:?}"
    );
}

#[test]
fn the_catalog_read_from_its_directory_is_the_built_in_one() {
    let files = botica_catalog::read_files(&rules_dir()).expect("the rules directory is read");
    let read = Catalog::from_files(&files).unwrap_or_else(|error| panic!("{error}"));

    let ids = |catalog: &Catalog| -> Vec<String> {
        catalog.rules().iter().map(|rule| rule.id.clone()).collect()
    };
    assert_eq!(ids(&read), ids(&embedded()));
    assert_eq!(read.families(), embedded().families());
    let yaml = read
        .resolve("log-no-print")
        .expect("log-no-print")
        .implementations[0]
        .source
        .clone();
    assert!(yaml.is_some_and(|text| text.starts_with("id: log-no-print\n")));
}
