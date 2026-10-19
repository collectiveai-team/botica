mod common;

use botica_catalog::{Catalog, SourceFile};
use common::{catalog, family, file, issues, rule, rule_toml, NATIVE_ALL, README};

fn rule_with_status(slug: &str, code: &str, status: &str) -> Vec<SourceFile> {
    let toml = rule_toml(slug, code, "", NATIVE_ALL)
        .replace("status = \"stable\"", &format!("status = \"{status}\""));
    vec![
        file(&format!("structure/{slug}/rule.toml"), &toml),
        file(&format!("structure/{slug}/README.md"), README),
    ]
}

fn history(events: &str) -> Vec<SourceFile> {
    vec![file("history.toml", events)]
}

fn event(version: &str, rule: &str, status: &str) -> String {
    format!("[[event]]\nversion = \"{version}\"\nrule = \"{rule}\"\nstatus = \"{status}\"\n\n")
}

fn load(rule_status: &str, events: &[String]) -> Catalog {
    catalog(vec![
        vec![family("structure", "STRUC")],
        rule_with_status("one", "BTSTRUC001", rule_status),
        history(&events.concat()),
    ])
    .expect("a catalog with a history loads")
}

fn lifecycle(loaded: &Catalog) -> (Option<String>, Option<String>) {
    let found = loaded.lifecycle(loaded.resolve("one").expect("the rule is there"));
    (found.added, found.since)
}

#[test]
fn a_rule_was_added_in_its_first_event_and_has_its_status_since_the_event_that_set_it() {
    let loaded = load(
        "deprecated",
        &[
            event("0.8.0", "one", "preview"),
            event("0.9.0", "one", "stable"),
            event("0.11.0", "one", "deprecated"),
        ],
    );

    assert_eq!(
        lifecycle(&loaded),
        (Some("0.8.0".to_owned()), Some("0.11.0".to_owned()))
    );
}

#[test]
fn a_rule_that_never_changed_status_has_it_since_the_release_that_added_it() {
    let loaded = load("stable", &[event("0.8.0", "one", "stable")]);

    assert_eq!(
        lifecycle(&loaded),
        (Some("0.8.0".to_owned()), Some("0.8.0".to_owned()))
    );
}

#[test]
fn a_rule_that_goes_back_to_a_status_has_it_since_the_last_time_it_got_it() {
    let loaded = load(
        "stable",
        &[
            event("0.8.0", "one", "stable"),
            event("0.9.0", "one", "deprecated"),
            event("0.10.0", "one", "stable"),
        ],
    );

    assert_eq!(
        lifecycle(&loaded),
        (Some("0.8.0".to_owned()), Some("0.10.0".to_owned()))
    );
}

#[test]
fn a_rule_with_no_event_is_not_released_yet() {
    let loaded = load("preview", &[]);

    assert_eq!(lifecycle(&loaded), (None, None));
}

#[test]
fn a_status_that_no_event_has_recorded_yet_is_not_released_yet_either() {
    let loaded = load("deprecated", &[event("0.8.0", "one", "stable")]);

    assert_eq!(lifecycle(&loaded), (Some("0.8.0".to_owned()), None));
}

#[test]
fn a_catalog_with_no_history_file_loads_and_releases_nothing() {
    let loaded = catalog(vec![
        vec![family("structure", "STRUC")],
        rule("structure", "one", "BTSTRUC001"),
    ])
    .expect("a project catalog has no history");

    assert_eq!(lifecycle(&loaded), (None, None));
}

fn rejected(events: &[String]) -> String {
    issues(vec![
        vec![family("structure", "STRUC")],
        rule_with_status("one", "BTSTRUC001", "stable"),
        history(&events.concat()),
    ])
}

#[test]
fn an_event_must_name_a_rule_of_the_catalog() {
    let message = rejected(&[event("0.8.0", "ghost", "stable")]);

    assert!(message.contains("history.toml"), "{message}");
    assert!(message.contains("ghost"), "{message}");
}

#[test]
fn the_version_of_an_event_must_look_like_a_release() {
    for bad in ["0.8", "v0.8.0", "0.8.x", ""] {
        let message = rejected(&[event(bad, "one", "stable")]);
        assert!(message.contains("version"), "{bad}: {message}");
    }
}

#[test]
fn the_events_never_go_back_to_an_older_version() {
    let message = rejected(&[
        event("0.9.0", "one", "preview"),
        event("0.8.0", "one", "stable"),
    ]);

    assert!(message.contains("0.8.0"), "{message}");
    assert!(message.contains("older"), "{message}");
}

#[test]
fn an_event_must_change_the_status_of_the_rule() {
    let message = rejected(&[
        event("0.8.0", "one", "stable"),
        event("0.9.0", "one", "stable"),
    ]);

    assert!(message.contains("0.9.0"), "{message}");
    assert!(message.contains("already"), "{message}");
}

#[test]
fn an_event_has_no_other_key() {
    let message = rejected(&[
        "[[event]]\nversion = \"0.8.0\"\nrule = \"one\"\nstatus = \"stable\"\nnote = \"x\"\n"
            .to_owned(),
    ]);

    assert!(message.contains("note"), "{message}");
}

#[test]
fn since_is_no_longer_a_key_of_a_rule() {
    let toml = rule_toml("one", "BTSTRUC001", "since = \"0.8.0\"", NATIVE_ALL);
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        vec![
            file("structure/one/rule.toml", &toml),
            file("structure/one/README.md", README),
        ],
    ]);

    assert!(message.contains("since"), "{message}");
}
