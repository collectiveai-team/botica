//! What the run line of `botica lint --record` says (ADR 0019 decisions 3 and 6): its mode, its
//! origin, its commit and branch, and the rules that it evaluated.

mod common;

use common::record::{findings, git, lint, lint_with, project, repository, runs, UNUSED};
use common::{project_in, stderr, write};
use serde_json::Value;

#[test]
fn the_rules_of_a_run_count_the_hits_of_each_rule_evaluated_and_list_those_with_none() {
    let dir = project("rules", None);

    lint(&dir, &["--record"]);

    let rules = &runs(&dir)[0]["rules"];
    assert_eq!(rules["unused-suppression"]["hits"], 1, "{rules}");
    assert_eq!(rules["unused-suppression"]["baselined"], 0, "{rules}");
    assert_eq!(rules["unused-suppression"]["hash"], Value::Null, "{rules}");
    assert_eq!(rules["no-utils"]["hits"], 0, "{rules}");
    assert!(
        rules.get("conventional-commits").is_none(),
        "a rule that is off is not evaluated: {rules}"
    );
}

#[test]
fn a_rule_that_the_plan_gave_no_file_is_absent() {
    let dir = project_in("lint-record", "no-file", None);
    write(&dir, "notes.txt", "nothing for the rules of python\n");

    lint(&dir, &["--record", "notes.txt"]);

    let rules = &runs(&dir)[0]["rules"];
    assert!(rules.get("no-utils").is_none(), "{rules}");
    assert!(rules.get("unused-suppression").is_some(), "{rules}");
}

#[test]
fn the_history_is_written_under_the_root_whatever_the_working_directory() {
    let dir = project("from-a-subdirectory", None);
    write(&dir, "src/b.py", UNUSED);

    lint(&dir.join("src"), &["--record"]);

    assert!(!dir.join("src").join(".botica").exists());
    let found = findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["path"], "src/b.py");
}

#[test]
fn the_mode_says_what_the_run_read() {
    let dir = project("paths", None);

    lint(&dir, &["--record", "a.py"]);
    lint(&dir, &["--record", "."]);

    let runs = runs(&dir);
    assert_eq!(runs[0]["mode"], "paths");
    assert_eq!(runs[1]["mode"], "full");
}

#[test]
fn the_origin_is_ci_before_hook_before_cli() {
    let dir = project("origin", None);

    lint_with(&dir, &[("PRE_COMMIT", "1")], &["--record"]);
    lint_with(
        &dir,
        &[("PRE_COMMIT", "1"), ("GITHUB_ACTIONS", "true")],
        &["--record"],
    );
    lint_with(&dir, &[("GITLAB_CI", "true")], &["--record"]);

    let origins: Vec<Value> = runs(&dir).iter().map(|run| run["origin"].clone()).collect();
    assert_eq!(origins, ["hook", "ci", "ci"]);
}

#[test]
fn a_run_in_git_records_the_commit_and_the_branch() {
    let dir = repository("git");
    let sha = git(&dir, &["rev-parse", "HEAD"]);

    lint(&dir, &["--record"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["commit"], sha.as_str());
    assert_eq!(run["head"], sha.as_str());
    assert_eq!(run["branch"], "topic");
}

#[test]
fn outside_git_the_commit_and_the_head_are_null() {
    let dir = project("no-git", None);
    let above = dir.parent().expect("has a parent");

    // The project sits inside the repository of botica: the ceiling keeps git from finding it.
    lint_with(
        &dir,
        &[("GIT_CEILING_DIRECTORIES", above.to_str().expect("UTF-8"))],
        &["--record"],
    );

    let run = &runs(&dir)[0];
    assert_eq!(run["commit"], Value::Null);
    assert_eq!(run["head"], Value::Null);
}

#[test]
fn on_a_pull_request_of_github_the_branch_and_the_head_come_from_the_event() {
    let dir = repository("github");
    let head = "a".repeat(40);
    write(
        &dir,
        "event.json",
        &format!("{{\"pull_request\":{{\"head\":{{\"sha\":\"{head}\"}}}}}}"),
    );
    let event = dir.join("event.json");

    lint_with(
        &dir,
        &[
            ("GITHUB_ACTIONS", "true"),
            ("GITHUB_EVENT_NAME", "pull_request"),
            ("GITHUB_EVENT_PATH", event.to_str().expect("UTF-8")),
            ("GITHUB_HEAD_REF", "feature"),
        ],
        &["--record", "a.py"],
    );

    let run = &runs(&dir)[0];
    assert_eq!(run["head"], head.as_str());
    assert_eq!(run["branch"], "feature");
    assert_eq!(run["commit"], git(&dir, &["rev-parse", "HEAD"]).as_str());
}

#[test]
fn a_staged_run_records_the_mode_staged_and_the_findings_on_the_added_lines() {
    let dir = repository("staged");
    write(&dir, "b.py", UNUSED);
    git(&dir, &["add", "b.py"]);

    lint(&dir, &["--record", "--staged"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "staged");
    let found = findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["path"], "b.py");
}

#[test]
fn a_diff_run_records_the_mode_diff() {
    let dir = repository("diff");

    lint(&dir, &["--record", "--diff", "HEAD"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "diff");
    assert!(findings(&dir).is_empty());
}

#[test]
fn a_message_run_has_empty_paths_and_counts_the_rules_that_checked_a_message() {
    let dir = project(
        "message",
        Some("preview = true\nselect = [\"conventional-commits\"]\n"),
    );
    write(&dir, "MSG", "Not conventional\n");

    lint(&dir, &["--record", "--commit-msg", "MSG"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "message");
    assert_eq!(run["rules"]["conventional-commits"]["hits"], 1, "{run}");
    let found = findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["path"], "");
    assert_eq!(found[0]["line"], 0);
}

#[test]
fn the_baseline_count_holds_the_findings_that_the_baseline_hides() {
    let config = "preview = true\nextend-select = [\"cognitive-complexity\"]\n\
                  baseline = \"botica-baseline.toml\"\n\n[rules.cognitive-complexity]\nmax = 1\n";
    let dir = project("baselined", Some(config));
    write(
        &dir,
        "src/c.py",
        "def nested(xs):\n    for x in xs:\n        if x:\n            pass\n",
    );
    let written = lint(&dir, &["--write-baseline"]);
    assert_eq!(written.status.code(), Some(0), "{}", stderr(&written));

    lint(&dir, &["--record"]);

    let rules = &runs(&dir)[0]["rules"];
    assert_eq!(rules["cognitive-complexity"]["baselined"], 1, "{rules}");
    assert_eq!(rules["cognitive-complexity"]["hits"], 0, "{rules}");
    assert!(findings(&dir)
        .iter()
        .all(|finding| finding["slug"] != "cognitive-complexity"));
}

#[test]
fn a_run_with_the_result_cache_lists_the_same_rules_as_one_without() {
    let dir = project("cached", None);

    lint(&dir, &["--record", "--cache"]);
    lint(&dir, &["--record", "--cache"]);
    lint(&dir, &["--record"]);

    let runs = runs(&dir);
    assert_eq!(runs[0]["rules"], runs[1]["rules"]);
    assert_eq!(runs[1]["rules"], runs[2]["rules"]);
}
