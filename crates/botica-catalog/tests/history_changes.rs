//! What the releases after a version did to the rules, read from `history.toml` (ADR 0016).

mod common;

use botica_catalog::{Catalog, Change, ChangeKind, Release, SourceFile, Status};
use common::{catalog, family, file, rule_toml, NATIVE_ALL, README};

fn rule(slug: &str, code: &str, extra: &str) -> Vec<SourceFile> {
    vec![
        file(
            &format!("structure/{slug}/rule.toml"),
            &rule_toml(slug, code, extra, NATIVE_ALL),
        ),
        file(&format!("structure/{slug}/README.md"), README),
    ]
}

fn event(version: &str, rule: &str, fields: &[(&str, &str)]) -> String {
    let mut text = format!("[[event]]\nversion = \"{version}\"\nrule = \"{rule}\"\n");
    for (key, value) in fields {
        text.push_str(&format!("{key} = \"{value}\"\n"));
    }
    text + "\n"
}

/// `one` came in 0.8.0 as `old` in preview with BTSTRUC001, became stable in 0.9.0, and was renamed
/// to `one` with the code BTSTRUC002 in 0.10.0; `two` came in 0.10.0.
fn loaded() -> Catalog {
    let events = [
        event(
            "0.8.0",
            "old",
            &[("status", "preview"), ("code", "BTSTRUC001")],
        ),
        event("0.9.0", "old", &[("status", "stable")]),
        event(
            "0.10.0",
            "one",
            &[("renamed-from", "old"), ("code", "BTSTRUC002")],
        ),
        event(
            "0.10.0",
            "two",
            &[("status", "stable"), ("code", "BTSTRUC003")],
        ),
    ];
    catalog(vec![
        vec![family("structure", "STRUC")],
        rule("one", "BTSTRUC002", "aliases = [\"old\", \"BTSTRUC001\"]"),
        rule("two", "BTSTRUC003", ""),
        vec![file("history.toml", &events.concat())],
    ])
    .expect("the catalog loads")
}

fn release(text: &str) -> Release {
    text.parse().expect("a version")
}

fn change(version: &str, rule: &str, code: &str, kind: ChangeKind) -> Change {
    Change {
        version: version.to_owned(),
        rule: rule.to_owned(),
        code: Some(code.to_owned()),
        kind,
    }
}

#[test]
fn lists_what_each_release_after_the_version_did_in_release_order() {
    let changes = loaded().changes(release("0.8.0"), release("0.10.0"));

    assert_eq!(
        changes,
        vec![
            change(
                "0.9.0",
                "old",
                "BTSTRUC001",
                ChangeKind::Status {
                    from: Some(Status::Preview),
                    to: Status::Stable,
                },
            ),
            change(
                "0.10.0",
                "one",
                "BTSTRUC002",
                ChangeKind::Renamed {
                    from: "old".to_owned(),
                },
            ),
            change(
                "0.10.0",
                "one",
                "BTSTRUC002",
                ChangeKind::Code {
                    from: Some("BTSTRUC001".to_owned()),
                    to: "BTSTRUC002".to_owned(),
                },
            ),
            change(
                "0.10.0",
                "two",
                "BTSTRUC003",
                ChangeKind::Added {
                    status: Status::Stable,
                },
            ),
        ]
    );
}

#[test]
fn the_first_event_of_a_rule_adds_it_and_its_code_is_no_change() {
    let changes = loaded().changes(release("0.7.0"), release("0.8.0"));

    assert_eq!(
        changes,
        vec![change(
            "0.8.0",
            "old",
            "BTSTRUC001",
            ChangeKind::Added {
                status: Status::Preview,
            },
        )]
    );
}

#[test]
fn a_release_after_the_last_one_or_after_the_upper_bound_lists_nothing() {
    let catalog = loaded();

    assert!(catalog
        .changes(release("0.10.0"), release("0.18.0"))
        .is_empty());
    assert!(catalog
        .changes(release("0.20.0"), release("0.18.0"))
        .is_empty());
    assert!(catalog
        .changes(release("0.7.0"), release("0.8.5"))
        .iter()
        .all(|change| change.version == "0.8.0"));
}

#[test]
fn a_removed_rule_names_the_rule_that_replaces_it() {
    let catalog = Catalog::embedded().expect("the built-in catalog loads");

    let changes = catalog.changes(release("0.9.0"), release("0.10.0"));

    let removed = changes
        .iter()
        .find(|change| change.rule == "no-console-log")
        .expect("0.10.0 removed no-console-log");
    assert_eq!(
        removed.kind,
        ChangeKind::Removed {
            replaced_by: Some("log-no-print".to_owned()),
        }
    );
}

#[test]
fn a_version_is_three_numbers() {
    assert_eq!(release("0.13.0").to_string(), "0.13.0");
    assert!(release("1.2.10") > release("1.2.9"));
    for bad in ["0.13", "v0.13.0", "0.13.x", "0.13.0.1", "", "1..2"] {
        let error = bad.parse::<Release>().expect_err(bad);
        assert!(error.contains("0.13.0"), "{bad}: {error}");
    }
}
