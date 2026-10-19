//! A judgment rule that an active deterministic rule covers (`covered-by`) is left out of what the
//! judges read, and `botica rules` says so.

mod common;

use std::path::Path;

use common::judgment::{project, read};
use common::{command, stderr, stdout};

fn run(dir: &Path, args: &[&str]) -> String {
    let output = command(dir, args);
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

fn block_lists_judge_it(dir: &Path) -> bool {
    run(
        dir,
        &[
            "rules",
            "export",
            "--format",
            "agents-md",
            "--output",
            "AGENTS.md",
        ],
    );
    read(dir, "AGENTS.md").contains("`acme/judge-it`")
}

fn rubric_ids(dir: &Path) -> Vec<String> {
    run(dir, &["rules", "export", "--format", "abide"]);
    let rubric: serde_json::Value =
        serde_json::from_str(&read(dir, ".abide/rubric.json")).expect("the rubric");
    rubric["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .filter_map(|rule| rule["id"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn a_judgment_rule_covered_by_an_active_rule_is_left_out_of_agents_md_and_the_rubric() {
    let dir = project("judgment-covered", "covered", "");

    assert!(!block_lists_judge_it(&dir), "{}", read(&dir, "AGENTS.md"));
    assert_eq!(rubric_ids(&dir), Vec::<String>::new());
}

#[test]
fn a_judgment_rule_whose_cover_is_not_active_is_exported() {
    let dir = project(
        "judgment-covered",
        "uncovered",
        "ignore = [\"acme/no-eval\"]\n",
    );

    assert!(block_lists_judge_it(&dir), "{}", read(&dir, "AGENTS.md"));
    assert_eq!(rubric_ids(&dir), ["acme/judge-it"]);
}

#[test]
fn rules_marks_a_judgment_rule_and_the_rules_that_cover_it() {
    let dir = project("judgment-covered", "listing", "");

    let text = run(&dir, &["rules"]);
    let line = text
        .lines()
        .find(|line| line.contains("acme/judge-it"))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(
        line.ends_with("; judged outside lint, covered by `acme/no-eval`, left out of the exports"),
        "{line}"
    );

    let document: serde_json::Value =
        serde_json::from_str(&run(&dir, &["rules", "--json"])).expect("JSON");
    let rule = |slug: &str| {
        document["rules"]
            .as_array()
            .expect("rules")
            .iter()
            .find(|rule| rule["rule"] == slug)
            .cloned()
            .expect("the rule")
    };
    assert_eq!(
        rule("acme/judge-it")["covered_by"],
        serde_json::json!(["acme/no-eval"])
    );
    assert_eq!(rule("acme/no-eval")["covered_by"], serde_json::json!([]));
}

#[test]
fn rules_marks_a_judgment_rule_that_nothing_covers_as_judged_outside_lint() {
    let dir = project(
        "judgment-covered",
        "listing-uncovered",
        "ignore = [\"acme/no-eval\"]\n",
    );

    let text = run(&dir, &["rules"]);
    let line = text
        .lines()
        .find(|line| line.contains("acme/judge-it"))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(line.ends_with("; judged outside lint"), "{line}");
}
