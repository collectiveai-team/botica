//! Which rules a run of `botica lint --record` lists as evaluated, and the paths it records (ADR
//! 0019 decisions 3, 4 and 6), for extension rules, project rules, judged runs and diffs.

mod common;

use common::extension::{add_rule, folder};
use common::record::{findings, git, lint, lint_with, project, repository, runs, UNUSED};
use common::{stderr, write};
use serde_json::Value;
use std::path::Path;

/// A project with the extension rules `acme/by-sh` and `acme/other`, which report every file.
fn extension_project(name: &str) -> std::path::PathBuf {
    let dir = common::extension::project(&format!("record-{name}"), "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_rule(&dir, "other", "ACMESTRUC002", "", "acme/other");
    dir
}

#[test]
fn an_extension_rule_that_finished_is_evaluated_and_one_that_failed_is_absent() {
    let dir = extension_project("extension-failed");
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), "exit 3\n");

    let output = lint(&dir, &["--record", "."]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let rules = &runs(&dir)[0]["rules"];
    assert_eq!(rules["acme/other"]["hits"], 1, "{rules}");
    assert!(rules.get("acme/by-sh").is_none(), "{rules}");
}

#[test]
fn an_extension_rule_that_reported_and_then_failed_is_absent_with_its_findings_recorded() {
    let dir = extension_project("extension-late");
    let script = std::fs::read_to_string(dir.join(folder("by-sh")).join("ext.sh"))
        .expect("the script")
        .replace(
            r#"*'"shutdown"'*) printf"#,
            r#"*'"shutdown"'*) exit 3; printf"#,
        );
    assert!(script.contains("exit 3;"), "{script}");
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), &script);

    lint(&dir, &["--record", "."]);

    let rules = &runs(&dir)[0]["rules"];
    assert!(rules.get("acme/by-sh").is_none(), "{rules}");
}

#[test]
fn an_active_rule_of_project_scope_is_evaluated_in_every_run_that_lints_files() {
    let dir = project("project-scope", None);
    write(&dir, "notes.txt", "x\n");

    lint(&dir, &["--record", "notes.txt"]);

    let rules = &runs(&dir)[0]["rules"];
    assert_eq!(rules["settings-env-schema"]["hits"], 0, "{rules}");
}

#[test]
fn a_run_with_no_path_from_a_subdirectory_records_the_mode_paths() {
    let dir = project("from-sub", None);
    write(&dir, "sub/b.py", UNUSED);

    lint(&dir.join("sub"), &["--record"]);

    assert_eq!(runs(&dir)[0]["mode"], "paths");
}

#[test]
fn a_finding_outside_the_root_records_an_empty_path() {
    let dir = project("outside", None);
    let outside = dir
        .parent()
        .expect("has a parent")
        .join("outside-of-record");
    write(&outside, "o.py", UNUSED);

    lint(&dir, &["--record", "../outside-of-record/o.py"]);

    let found = findings(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0]["path"], "");
    assert_eq!(found[0]["line"], 0);
}

#[test]
fn record_and_write_baseline_exclude_each_other() {
    let dir = project("write-baseline", Some("baseline = \"b.toml\"\n"));

    let output = lint(&dir, &["--record", "--write-baseline"]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    assert!(
        stderr(&output).contains("--write-baseline"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_judged_run_records_the_mode_judged_and_the_rules_of_its_findings() {
    let dir = project(
        "judged",
        Some("preview = true\nselect = [\"test-through-interface\"]\n"),
    );
    write(&dir, "tests/test_a.py", "def test_a():\n    assert a._x\n");
    write(
        &dir,
        "judge.sarif",
        r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "test-through-interface", "message": {"text": "m"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "tests/test_a.py"}, "region": {"startLine": 2}}}]}]}]}"#,
    );

    lint(&dir, &["--record", "--judged", "judge.sarif"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "judged");
    assert_eq!(run["rules"]["test-through-interface"]["hits"], 1, "{run}");
    assert_eq!(findings(&dir)[0]["path"], "tests/test_a.py");
}

#[test]
fn a_diff_run_counts_in_baselined_only_the_hidden_findings_on_the_lines_it_adds() {
    let config = "preview = true\nextend-select = [\"cognitive-complexity\"]\n\
                  baseline = \"botica-baseline.toml\"\n\n[rules.cognitive-complexity]\nmax = 1\n";
    let dir = repository("baselined-diff");
    let nested = "def nested(xs):\n    for x in xs:\n        if x:\n            pass\n";
    write(&dir, "botica.toml", config);
    write(&dir, "old.py", nested);
    let written = lint(&dir, &["--write-baseline"]);
    assert_eq!(written.status.code(), Some(0), "{}", stderr(&written));
    git(&dir, &["add", "--all"]);
    git(&dir, &["commit", "--quiet", "-m", "baseline"]);
    write(&dir, "new.py", &nested.replace("nested", "other"));
    write_baseline_with_both(&dir);

    lint(&dir, &["--record", "--diff", "HEAD"]);

    let rules = &runs(&dir)[0]["rules"];
    assert_eq!(rules["cognitive-complexity"]["baselined"], 1, "{rules}");
}

/// Adds `new.py` to the baseline, so that it hides the findings of both files.
fn write_baseline_with_both(dir: &Path) {
    let written = lint_with(dir, &[], &["--write-baseline"]);
    assert_eq!(written.status.code(), Some(0), "{}", stderr(&written));
    let text = std::fs::read_to_string(dir.join("botica-baseline.toml")).expect("the baseline");
    assert_eq!(text.matches("[[finding]]").count(), 2, "{text}");
}

#[test]
fn a_run_with_the_ast_grep_rules_lists_them_and_one_without_does_not() {
    let dir = project("astgrep", None);
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_botica"));
    command
        .args(["lint", "--record"])
        .current_dir(&dir)
        .env_remove("BOTICA_HISTORY_KEY")
        .env_remove("GITHUB_ACTIONS")
        .env_remove("GITLAB_CI")
        .env_remove("PRE_COMMIT");
    command.output().expect("run botica");
    lint(&dir, &["--record"]);

    let runs = runs(&dir);
    let with = runs[0]["rules"].as_object().expect("rules").len();
    let without = runs[1]["rules"].as_object().expect("rules").len();
    assert!(
        with > without,
        "{} against {}",
        runs[0]["rules"],
        runs[1]["rules"]
    );
    assert_eq!(
        runs[0]["rules"].get("io-error"),
        runs[1]["rules"].get("io-error")
    );
    assert_ne!(runs[0]["rules"], Value::Null);
}
