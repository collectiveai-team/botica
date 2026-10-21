//! `botica recipe check` reports how a project differs from a recipe: the rules that are not
//! active and the exports that are not current.

mod common;

use std::path::Path;
use std::process::Command;

use common::recipes::{add_recipe, judgment_rule, project, recipe, SELECT_BASE};
use common::refs::{failed, ok};
use common::write;

const START: &str = "<!-- botica:rules -->";
const END: &str = "<!-- /botica:rules -->";

fn export_abide(dir: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["rules", "export", "--format", "abide"])
        .current_dir(dir)
        .output()
        .expect("run botica");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A recipe of the project with the rules of `base` and none of its steps.
fn base_rules(dir: &Path) {
    add_recipe(
        dir,
        "base-rules",
        "rules = [\"file-size-guard\", \"no-notebooks\", \"no-artifacts-in-package\"]\n",
    );
}

#[test]
fn a_project_that_runs_every_rule_of_the_recipe_has_no_difference() {
    let dir = project("check-ok", None);
    base_rules(&dir);

    let text = ok(&recipe(
        &dir,
        &["check", "base-rules", "--select", SELECT_BASE],
    ));

    assert_eq!(
        text,
        "rule file-size-guard: ok\nrule no-notebooks: ok\nrule no-artifacts-in-package: ok\n\
         base-rules: 3 checked, 0 differ\n"
    );
}

#[test]
fn a_rule_that_the_selection_leaves_out_differs_and_the_command_fails() {
    let dir = project("check-not-selected", None);
    base_rules(&dir);

    let text = failed(
        &recipe(
            &dir,
            &["check", "base-rules", "--select", "file-size-guard"],
        ),
        1,
    );

    assert!(text.contains("rule file-size-guard: ok\n"), "{text}");
    assert!(
        text.contains("rule no-notebooks: differs: not selected\n"),
        "{text}"
    );
    assert!(
        text.contains("rule no-artifacts-in-package: differs: not selected\n"),
        "{text}"
    );
    assert!(
        text.ends_with("base-rules: 3 checked, 2 differ\n"),
        "{text}"
    );
}

#[test]
fn a_rule_that_the_configuration_switches_off_says_why() {
    let dir = project(
        "check-ignored",
        Some("[rules.no-notebooks]\nenabled = false\n"),
    );
    base_rules(&dir);

    let text = failed(
        &recipe(&dir, &["check", "base-rules", "--select", SELECT_BASE]),
        1,
    );

    assert!(
        text.contains("rule no-notebooks: differs: `enabled = false` in the configuration\n"),
        "{text}"
    );
    assert!(
        text.ends_with("base-rules: 3 checked, 1 differs\n"),
        "{text}"
    );
}

#[test]
fn a_rule_that_is_ignored_on_the_command_line_says_which_selector_did() {
    let dir = project("check-ignore-flag", None);

    let text = failed(
        &recipe(
            &dir,
            &[
                "check",
                "base",
                "--select",
                SELECT_BASE,
                "--ignore",
                "no-notebooks",
            ],
        ),
        1,
    );

    assert!(
        text.contains("rule no-notebooks: differs: ignored by `no-notebooks`\n"),
        "{text}"
    );
}

#[test]
fn a_recipe_is_checked_with_the_rules_of_what_it_extends() {
    let dir = project("check-extends", None);
    add_recipe(
        &dir,
        "parent",
        "rules = [\"file-size-guard\", \"no-notebooks\"]\n",
    );
    add_recipe(
        &dir,
        "child",
        "extends = [\"parent\"]\nrules = [\"no-utils\"]\n",
    );
    let all = "file-size-guard,no-notebooks,no-utils";

    let text = ok(&recipe(&dir, &["check", "child", "--select", all]));

    assert_eq!(
        text,
        "rule file-size-guard: ok\nrule no-notebooks: ok\nrule no-utils: ok\n\
         child: 3 checked, 0 differ\n"
    );
}

#[test]
fn a_rule_that_is_not_in_the_catalog_differs() {
    let dir = project("check-ghost", None);
    add_recipe(&dir, "mine", "rules = [\"ghost\"]\n");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("rule ghost: differs: not in the catalog\n"),
        "{text}"
    );
}

#[test]
fn a_rule_is_found_by_its_code_or_its_slug() {
    let dir = project("check-code", None);
    add_recipe(&dir, "mine", "rules = [\"BTSTRUC001\"]\n");

    let text = ok(&recipe(
        &dir,
        &["check", "mine", "--select", "file-size-guard"],
    ));

    assert!(text.contains("rule BTSTRUC001: ok\n"), "{text}");
}

#[test]
fn a_rule_listed_as_judgment_that_is_not_one_differs() {
    let dir = project("check-judgment", None);
    add_recipe(&dir, "mine", "judgment = [\"file-size-guard\"]\n");

    let text = failed(
        &recipe(&dir, &["check", "mine", "--select", "file-size-guard"]),
        1,
    );

    assert!(
        text.contains("judgment rule file-size-guard: differs: is not a judgment rule\n"),
        "{text}"
    );
}

#[test]
fn the_block_of_agents_md_is_an_export_that_must_be_current() {
    let dir = project("check-agents-md", None);
    add_recipe(&dir, "mine", "exports = [\"agents-md\"]\n");

    let missing = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        missing.contains("export agents-md: differs: cannot read `AGENTS.md`"),
        "{missing}"
    );

    write(
        &dir,
        "AGENTS.md",
        &format!("# Notes\n\n{START}\n- stale\n{END}\n"),
    );
    let stale = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        stale.contains("export agents-md: differs: the block of `AGENTS.md` differs"),
        "{stale}"
    );

    write(&dir, "AGENTS.md", &format!("# Notes\n\n{START}\n{END}\n"));
    let current = ok(&recipe(&dir, &["check", "mine"]));
    assert_eq!(current, "export agents-md: ok\nmine: 1 checked, 0 differ\n");
}

#[test]
fn the_rubric_of_abide_is_an_export_that_must_be_current() {
    let dir = project("check-abide", None);
    add_recipe(&dir, "mine", "exports = [\"abide\"]\n");
    write(&dir, "AGENTS.md", &format!("{START}\n{END}\n"));

    let missing = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        missing.contains("export abide: differs: `.abide/rubric.json` differs"),
        "{missing}"
    );

    export_abide(&dir);
    let current = ok(&recipe(&dir, &["check", "mine"]));
    assert!(current.contains("export abide: ok\n"), "{current}");
}

#[test]
fn what_a_recipe_lists_is_checked_in_order_rules_judgment_then_exports() {
    let dir = project("check-order", None);
    add_recipe(
        &dir,
        "mine",
        "rules = [\"ghost\"]\njudgment = [\"phantom\"]\nexports = [\"agents-md\"]\n",
    );

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    let ghost = text.find("rule ghost").expect("rule");
    let phantom = text.find("judgment rule phantom").expect("judgment");
    let export = text.find("export agents-md").expect("export");
    assert!(ghost < phantom && phantom < export, "{text}");
    assert!(text.ends_with("mine: 3 checked, 3 differ\n"), "{text}");
}

#[test]
fn a_recipe_that_does_not_exist_is_an_error() {
    let dir = project("check-unknown", None);

    let text = failed(&recipe(&dir, &["check", "nope"]), 2);

    assert!(text.contains("no recipe `nope`"), "{text}");
}

#[test]
fn a_judgment_rule_is_listed_under_judgment_and_a_rule_under_rules() {
    let dir = project("check-judgment-kind", None);
    judgment_rule(&dir);
    add_recipe(&dir, "right", "judgment = [\"acme/judge-it\"]\n");
    add_recipe(&dir, "wrong", "rules = [\"acme/judge-it\"]\n");
    let on = ["--select", "acme/judge-it", "--preview"];

    let right = ok(&recipe(&dir, &[&["check", "right"], &on[..]].concat()));
    assert!(
        right.contains("judgment rule acme/judge-it: ok\n"),
        "{right}"
    );

    let wrong = failed(&recipe(&dir, &[&["check", "wrong"], &on[..]].concat()), 1);
    assert!(
        wrong.contains(
            "rule acme/judge-it: differs: is a judgment rule: list it under `judgment`\n"
        ),
        "{wrong}"
    );
}

#[test]
fn a_judgment_rule_that_does_not_run_says_why() {
    let dir = project("check-judgment-off", None);
    judgment_rule(&dir);
    add_recipe(&dir, "right", "judgment = [\"acme/judge-it\"]\n");

    let text = failed(&recipe(&dir, &["check", "right"]), 1);

    assert!(
        text.contains("judgment rule acme/judge-it: differs: not selected\n"),
        "{text}"
    );
}
