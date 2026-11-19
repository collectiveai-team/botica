//! What a run of `botica lint --record` counts in `suppressed` for each rule (ADR 0019 decision
//! 3): the findings that a comment silenced, whichever way the run reached them.

mod common;

use common::extension::add_rule;
use common::record::{git, lint, project, repository, runs};
use common::write;
use serde_json::Value;

const SILENCED: &str = "import os\nx = 1  # botica: ignore[no-utils, unused-suppression]\ny = 2\n";

fn rules_of_the_first_run(dir: &std::path::Path) -> Value {
    runs(dir)[0]["rules"].clone()
}

#[test]
fn a_finding_that_a_comment_silences_counts_as_suppressed_and_not_as_a_hit() {
    let dir = project("line", None);
    write(&dir, "a.py", SILENCED);

    lint(&dir, &["--record"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["unused-suppression"]["suppressed"], 1, "{rules}");
    assert_eq!(rules["unused-suppression"]["hits"], 0, "{rules}");
    assert_eq!(rules["unused-suppression"]["baselined"], 0, "{rules}");
}

#[test]
fn a_rule_with_no_suppression_lists_suppressed_as_zero() {
    let dir = project("none", None);

    lint(&dir, &["--record"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["no-utils"]["suppressed"], 0, "{rules}");
    assert_eq!(rules["unused-suppression"]["suppressed"], 0, "{rules}");
}

#[test]
fn an_ignore_file_comment_counts_the_finding_of_the_file() {
    let dir = project("file", None);
    write(&dir, "utils.py", "# botica: ignore-file[no-utils]\nx = 1\n");

    lint(&dir, &["--record"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["no-utils"]["suppressed"], 1, "{rules}");
    assert_eq!(rules["no-utils"]["hits"], 0, "{rules}");
}

#[test]
fn a_suppressed_finding_has_no_finding_line() {
    let dir = project("no-line", None);
    write(&dir, "a.py", SILENCED);

    lint(&dir, &["--record"]);

    assert!(common::record::findings(&dir).is_empty());
}

#[test]
fn a_run_with_the_result_cache_counts_the_same_suppressions_as_one_without() {
    let dir = project("cached", None);
    write(&dir, "a.py", SILENCED);

    lint(&dir, &["--record", "--cache"]);
    lint(&dir, &["--record", "--cache"]);
    lint(&dir, &["--record"]);

    let runs = runs(&dir);
    assert_eq!(runs[0]["rules"]["unused-suppression"]["suppressed"], 1);
    assert_eq!(runs[0]["rules"], runs[1]["rules"]);
    assert_eq!(runs[1]["rules"], runs[2]["rules"]);
}

#[test]
fn a_diff_run_counts_only_the_suppressions_on_the_lines_it_adds() {
    let dir = repository("diff");
    let comment = "# botica: ignore[no-utils, unused-suppression]";
    write(
        &dir,
        "a.py",
        &format!("import os\nx = 1  {comment}\ny = 2\nz = 3  {comment}\n"),
    );
    git(&dir, &["add", "a.py"]);
    git(&dir, &["commit", "--quiet", "-m", "two comments"]);
    write(
        &dir,
        "a.py",
        &format!("import os\nx = 1  {comment}\ny = 2\nz = 4  {comment}\n"),
    );

    lint(&dir, &["--record", "--diff", "HEAD"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["unused-suppression"]["suppressed"], 1, "{rules}");
}

#[test]
fn an_extension_rule_counts_the_findings_that_a_comment_silences() {
    let dir = common::extension::project("record-suppressed", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, "app.py", "# botica: ignore-file[acme/by-sh]\nx = 1\n");

    lint(&dir, &["--record", "."]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["acme/by-sh"]["suppressed"], 1, "{rules}");
    assert_eq!(rules["acme/by-sh"]["hits"], 0, "{rules}");
}

#[test]
fn a_judged_run_counts_the_findings_that_an_ignore_file_comment_silences() {
    let dir = project(
        "judged",
        Some("preview = true\nselect = [\"test-through-interface\"]\n"),
    );
    write(
        &dir,
        "tests/test_a.py",
        "# botica: ignore-file[test-through-interface] -- reviewed\ndef test_a():\n    assert a._x\n",
    );
    write(
        &dir,
        "judge.sarif",
        r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "test-through-interface", "message": {"text": "m"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "tests/test_a.py"}, "region": {"startLine": 3}}}]}]}]}"#,
    );

    lint(&dir, &["--record", "--judged", "judge.sarif"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["test-through-interface"]["suppressed"], 1, "{rules}");
    assert_eq!(rules["test-through-interface"]["hits"], 0, "{rules}");
}

#[test]
fn a_file_reached_twice_counts_its_suppressions_once() {
    let dir = project("twice", None);
    write(&dir, "a.py", SILENCED);

    lint(&dir, &["--record", "a.py", "a.py", "."]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["unused-suppression"]["suppressed"], 1, "{rules}");
}

#[test]
fn a_rule_that_the_configuration_turns_off_at_a_path_counts_no_suppression_there() {
    let config = "[[overrides]]\nfiles = [\"legacy/**\"]\nignore = [\"unused-suppression\"]\n";
    let dir = project("override", Some(config));
    write(&dir, "legacy/a.py", SILENCED);

    lint(&dir, &["--record"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["unused-suppression"]["suppressed"], 0, "{rules}");
}

#[test]
fn a_rule_of_project_scope_counts_the_findings_that_a_comment_silences() {
    let dir = common::record::project("project-scope", Some(PROJECT_CONFIG));
    add_weakened_rule(&dir);
    write(
        &dir,
        "pyproject.toml",
        "[tool.pyrefly]\npreset = \"basic\"  # botica: ignore[external-config-weakened] -- moves to strict in Q1\n",
    );

    lint(&dir, &["--record", "--scope", "project"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(
        rules["external-config-weakened"]["suppressed"], 1,
        "{rules}"
    );
    assert_eq!(rules["external-config-weakened"]["hits"], 0, "{rules}");
}

const PROJECT_CONFIG: &str = "preview = true\n\
    extend-select = [\"acme/pyrefly-strict\", \"external-config-weakened\"]\n\
    ignore = [\"docstring-matches-signature\", \"ruff-heavy-imports-in-function\"]\n";

/// A catalog `acme` whose rule asks pyrefly for `preset = "strict"`.
fn add_weakened_rule(dir: &std::path::Path) {
    let rule = ".botica/rules/acme/checks/pyrefly-strict";
    write(dir, ".botica/rules/acme/catalog.toml", "code = \"ACME\"\n");
    write(
        dir,
        ".botica/rules/acme/checks/family.toml",
        "code = \"CHK\"\ndescription = \"The checks that another tool runs\"\n",
    );
    write(
        dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/pyrefly-strict\"\ncode = \"ACMECHK001\"\nsummary = \"s\"\n\
         status = \"preview\"\ndefault-enabled = false\ndefault-severity = \"error\"\n\n\
         [[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\ntool = \"pyrefly\"\n\
         docs = \"https://pyrefly.org/en/docs/configuration/\"\nconfig = \"config/pyproject.toml\"\n",
    );
    write(
        dir,
        &format!("{rule}/README.md"),
        "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
         ## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n",
    );
    write(
        dir,
        &format!("{rule}/config/pyproject.toml"),
        "[tool.pyrefly]\npreset = \"strict\"\n",
    );
}

#[test]
fn a_judged_diff_run_counts_only_the_suppressions_on_the_lines_it_adds() {
    let dir = repository("judged-diff");
    write(
        &dir,
        "tests/test_a.py",
        "# botica: ignore-file[test-through-interface] -- reviewed\ndef test_a():\n    assert a._x\n",
    );
    git(&dir, &["add", "tests"]);
    git(&dir, &["commit", "--quiet", "-m", "tests"]);
    write(
        &dir,
        "botica.toml",
        "preview = true\nselect = [\"test-through-interface\"]\n",
    );
    write(
        &dir,
        "judge.sarif",
        r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "test-through-interface", "message": {"text": "m"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "tests/test_a.py"}, "region": {"startLine": 3}}}]}]}]}"#,
    );

    lint(
        &dir,
        &["--record", "--judged", "judge.sarif", "--diff", "HEAD"],
    );

    let rules = rules_of_the_first_run(&dir);
    assert!(rules.get("test-through-interface").is_none(), "{rules}");
}

#[test]
fn the_entry_of_a_rule_has_exactly_hits_suppressed_baselined_and_hash() {
    let dir = project("keys", None);

    lint(&dir, &["--record"]);

    let entry = rules_of_the_first_run(&dir)["no-utils"].clone();
    let mut keys: Vec<&String> = entry.as_object().expect("an object").keys().collect();
    keys.sort();
    assert_eq!(keys, ["baselined", "hash", "hits", "suppressed"]);
    assert_eq!(runs(&dir)[0]["v"], 1);
}

#[test]
fn a_staged_run_counts_the_suppressions_of_the_staged_text() {
    let dir = repository("staged");
    write(&dir, "a.py", SILENCED);
    git(&dir, &["add", "a.py"]);

    lint(&dir, &["--record", "--staged"]);

    let run = &runs(&dir)[0];
    assert_eq!(run["mode"], "staged");
    assert_eq!(run["rules"]["unused-suppression"]["suppressed"], 1, "{run}");
}

#[test]
fn a_run_over_one_named_file_counts_its_suppressions() {
    let dir = project("named", None);
    write(&dir, "a.py", SILENCED);

    lint(&dir, &["--record", "a.py"]);

    let rules = rules_of_the_first_run(&dir);
    assert_eq!(rules["unused-suppression"]["suppressed"], 1, "{rules}");
}
