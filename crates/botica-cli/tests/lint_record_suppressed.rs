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
