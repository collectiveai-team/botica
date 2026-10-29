//! The codes and the ids that `history.toml` records, and the reuse of a published one (ADR 0016).

mod common;

use botica_catalog::{Catalog, SourceFile};
use common::{catalog, family, file, issues, rule_toml, NATIVE_ALL, README};

fn rule(slug: &str, code: &str, extra: &str) -> Vec<SourceFile> {
    vec![
        file(
            &format!("structure/{slug}/rule.toml"),
            &rule_toml(slug, code, extra, NATIVE_ALL),
        ),
        file(&format!("structure/{slug}/README.md"), README),
    ]
}

/// An `[[event]]` with the keys of `fields`, in their order.
fn event(version: &str, rule: &str, fields: &[(&str, &str)]) -> String {
    let mut text = format!("[[event]]\nversion = \"{version}\"\nrule = \"{rule}\"\n");
    for (key, value) in fields {
        text.push_str(&format!("{key} = \"{value}\"\n"));
    }
    text + "\n"
}

fn parts(rules: Vec<Vec<SourceFile>>, events: &[String]) -> Vec<Vec<SourceFile>> {
    let mut parts = vec![vec![family("structure", "STRUC")]];
    parts.extend(rules);
    parts.push(vec![file("history.toml", &events.concat())]);
    parts
}

fn load(rules: Vec<Vec<SourceFile>>, events: &[String]) -> Catalog {
    catalog(parts(rules, events)).expect("the catalog loads")
}

fn rejected(rules: Vec<Vec<SourceFile>>, events: &[String]) -> String {
    issues(parts(rules, events))
}

fn renamed_one() -> Vec<String> {
    vec![
        event(
            "0.8.0",
            "old",
            &[("status", "stable"), ("code", "BTSTRUC001")],
        ),
        event("0.13.0", "one", &[("renamed-from", "old")]),
    ]
}

#[test]
fn a_renamed_rule_was_added_in_the_first_event_of_its_old_id() {
    let loaded = load(
        vec![rule("one", "BTSTRUC001", "aliases = [\"old\"]")],
        &renamed_one(),
    );

    let lifecycle = loaded.lifecycle(loaded.resolve("one").expect("the rule is there"));

    assert_eq!(lifecycle.added.as_deref(), Some("0.8.0"));
    assert_eq!(lifecycle.since.as_deref(), Some("0.8.0"));
}

#[test]
fn an_event_after_a_rename_names_the_rule_by_its_new_id() {
    let mut events = renamed_one();
    events.push(event("0.14.0", "old", &[("status", "deprecated")]));

    let message = rejected(
        vec![rule("one", "BTSTRUC001", "aliases = [\"old\"]")],
        &events,
    );

    assert!(
        message.contains("`old` is `one` since `0.13.0`"),
        "{message}"
    );
}

#[test]
fn a_rename_names_an_id_that_an_event_before_it_names() {
    let message = rejected(
        vec![rule("one", "BTSTRUC001", "")],
        &[event("0.8.0", "one", &[("renamed-from", "ghost")])],
    );

    assert!(
        message.contains("no event before names `ghost`"),
        "{message}"
    );
}

#[test]
fn a_rename_leaves_no_rule_with_the_old_id() {
    let message = rejected(
        vec![rule("one", "BTSTRUC001", ""), rule("old", "BTSTRUC002", "")],
        &renamed_one(),
    );

    assert!(message.contains("still the id of a rule"), "{message}");
}

#[test]
fn a_code_event_changes_the_code_of_the_rule() {
    let message = rejected(
        vec![rule("one", "BTSTRUC001", "")],
        &[
            event("0.8.0", "one", &[("code", "BTSTRUC001")]),
            event("0.9.0", "one", &[("code", "BTSTRUC001")]),
        ],
    );

    assert!(
        message.contains("already has the code `BTSTRUC001`"),
        "{message}"
    );
}

#[test]
fn an_event_says_something() {
    let message = rejected(
        vec![rule("one", "BTSTRUC001", "")],
        &[event("0.8.0", "one", &[])],
    );

    assert!(
        message.contains("no status, code or renamed-from"),
        "{message}"
    );
}

#[test]
fn a_code_that_a_release_gave_to_a_rule_never_names_another_one() {
    let message = rejected(
        vec![rule("one", "BTSTRUC002", ""), rule("two", "BTSTRUC001", "")],
        &[
            event(
                "0.18.0",
                "one",
                &[("status", "stable"), ("code", "BTSTRUC001")],
            ),
            event("0.19.0", "one", &[("code", "BTSTRUC002")]),
        ],
    );

    assert!(
        message.contains("`BTSTRUC001` names `one` and `two`"),
        "{message}"
    );
}

#[test]
fn an_id_that_a_release_published_is_no_alias_of_another_rule() {
    let message = rejected(
        vec![
            rule("one", "BTSTRUC001", ""),
            rule("two", "BTSTRUC002", "aliases = [\"gone\"]"),
        ],
        &[
            event(
                "0.18.0",
                "gone",
                &[("status", "stable"), ("code", "BTSTRUC009")],
            ),
            event("0.19.0", "one", &[("renamed-from", "gone")]),
        ],
    );

    assert!(
        message.contains("`gone` names `one` and `two`"),
        "{message}"
    );
}

#[test]
fn the_reuses_that_the_history_records_by_0_17_1_stay() {
    let loaded = load(
        vec![rule("one", "BTSTRUC002", ""), rule("two", "BTSTRUC001", "")],
        &[
            event(
                "0.8.0",
                "one",
                &[("status", "stable"), ("code", "BTSTRUC001")],
            ),
            event("0.13.0", "one", &[("code", "BTSTRUC002")]),
            event(
                "0.13.0",
                "two",
                &[("status", "stable"), ("code", "BTSTRUC001")],
            ),
        ],
    );

    assert_eq!(
        loaded.resolve("BTSTRUC001").map(|rule| rule.id.as_str()),
        Some("two")
    );
}
