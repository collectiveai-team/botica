//! `botica recipe install` writes the rules of the recipes into the `select` of the configuration.

mod common;

use std::fs;

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::write;

const RULES: &str = "rules = [\"file-size-guard\", \"no-utils\"]\n";

#[test]
fn a_project_without_a_configuration_gets_a_botica_toml_that_selects_the_rules() {
    let dir = project("selection-add", None);
    add_recipe(&dir, "mine", RULES);

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "selection botica.toml: added\nmine: 1 added, 0 ok, 0 pending\n"
    );
    let config = fs::read_to_string(dir.join("botica.toml")).expect("read botica.toml");
    assert!(
        config.ends_with("select = [\n  \"file-size-guard\",\n  \"no-utils\",\n]\n"),
        "{config}"
    );
    let rules = ok(&recipe(&dir, &["check", "mine"]));
    assert!(rules.contains("rule no-utils: ok\n"), "{rules}");
}

#[test]
fn a_configuration_that_selects_every_rule_is_ok() {
    let dir = project(
        "selection-ok",
        Some("select = [\"no-utils\", \"file-size-guard\", \"no-notebooks\"]\n"),
    );
    add_recipe(&dir, "mine", RULES);

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "selection botica.toml: ok\nmine: 0 added, 1 ok, 0 pending\n"
    );
}

#[test]
fn a_configuration_that_lacks_a_rule_is_pending_and_left_as_it_is() {
    let dir = project("selection-pending", None);
    write(
        &dir,
        "pyproject.toml",
        "[tool.botica]\nselect = [\"file-size-guard\"]\n",
    );
    add_recipe(&dir, "mine", RULES);

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.starts_with(
            "selection botica.toml: pending: the configuration in `pyproject.toml` exists and its \
             `select` lacks no-utils\n"
        ),
        "{text}"
    );
    assert!(!dir.join("botica.toml").exists());
}
