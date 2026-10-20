//! `botica rules test --judge abide`, against a fake abide: a script that writes down how it was
//! called and prints a report of `abide audit --json`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const RULE: &str = "id = \"test-foo\"\ncode = \"BTTEST901\"\nsummary = \"s\"\nstatus = \"preview\"\n\
default-enabled = false\ndefault-severity = \"warning\"\n\n[[implementations]]\n\
kind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"WHEN\"\nwhy-not-deterministic = \"w\"\n\n\
[implementations.question]\ntype = \"boolean\"\ninstructions = \"Does the test read a private helper?\"\n";

const README: &str = "## Directive\n\nTests assert through the public seam.\n\n## Why\n\nw\n\n\
## Flag\n\nf\n\n## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
## Suppression\n\ns\n";

const EXPECTED: &str = r#"{"verdicts": [
  {"file": "valid/seam.py", "band": "clear", "max": 0.2},
  {"file": "invalid/private.py", "band": "act", "min": 0.9}
]}"#;

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create a directory");
    for entry in fs::read_dir(from).expect("read a directory").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy a file");
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("a directory");
    fs::write(path, text).expect("write a file");
}

/// A working directory with the catalog of this checkout under `cat/`, with the judgment rule
/// `test-foo`, judged on `when`, and a fake abide that prints `report`.
fn workdir(name: &str, when: &str, report: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-test")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/rules"),
        &dir.join("cat"),
    );
    // Without a rule named, `rules test` judges every judgment rule: here only `test-foo`.
    for family in ["writing", "process"] {
        fs::remove_dir_all(dir.join("cat").join(family))
            .expect("drop the judgment rules of the catalog");
    }
    let rule = dir.join("cat/testing/test-foo");
    write(&rule.join("rule.toml"), &RULE.replace("WHEN", when));
    write(&rule.join("README.md"), README);
    write(&rule.join("fixtures/expected.json"), EXPECTED);
    write(
        &rule.join("fixtures/valid/seam.py"),
        "assert add(1, 2) == 3\n",
    );
    write(
        &rule.join("fixtures/invalid/private.py"),
        "assert _helper() == 3\n",
    );
    write(
        &dir.join("abide.sh"),
        &format!(
            "printf '%s\\n' \"$@\" > called.txt\n\
             test -d .git || {{ echo 'not a git repository' >&2; exit 3; }}\n\
             cat <<'REPORT'\n{report}\nREPORT\nexit 1\n"
        ),
    );
    dir
}

fn report(seam: (&str, f64), private: (&str, f64)) -> String {
    format!(
        r#"{{"root": ".", "files": 2, "byFile": [
          {{"file": "invalid/private.py", "verdicts": [
            {{"ruleId": "test-foo", "probability": {}, "band": "{}"}},
            {{"ruleId": "a-global-rule", "probability": 0.99, "band": "act"}}]}},
          {{"file": "valid/seam.py", "verdicts": [
            {{"ruleId": "a-global-rule", "probability": 0.99, "band": "act"}},
            {{"ruleId": "test-foo", "probability": {}, "band": "{}"}}]}}]}}"#,
        private.1, private.0, seam.1, seam.0
    )
}

fn rules_test(dir: &Path, extra: &[&str]) -> Output {
    let abide = format!("sh {}", dir.join("abide.sh").display());
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args([
            "rules",
            "test",
            "--judge",
            "abide",
            "--catalog",
            "cat",
            "--abide",
        ])
        .arg(&abide)
        .args(extra)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn verdicts_as_expected_pass_and_the_repository_of_the_test_stays_to_be_read() {
    let dir = workdir("pass", "edit", &report(("clear", 0.1), ("act", 0.93)));

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "ok   test-foo valid/seam.py: clear 0.10\n\
         ok   test-foo invalid/private.py: act 0.93\n\
         2 of 2 fixtures as expected\n"
    );
    let judge = dir.join(".botica/judge");
    assert_eq!(
        fs::read_to_string(judge.join(".gitignore")).expect("ignored"),
        "*\n"
    );
    let repo = judge.join("test-foo");
    assert!(repo.join(".git").is_dir());
    assert_eq!(
        fs::read_to_string(repo.join("invalid/private.py")).expect("a fixture"),
        "assert _helper() == 3\n"
    );
    assert_eq!(
        fs::read_to_string(repo.join("called.txt")).expect("abide was called"),
        "audit\n--json\nvalid\ninvalid\n"
    );
    let rubric: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".abide/rubric.json")).expect("the rubric"),
    )
    .expect("JSON");
    assert_eq!(rubric["manual"], true);
    assert_eq!(rubric["sources"], serde_json::json!([]));
    assert_eq!(
        rubric["rules"],
        serde_json::json!([{
            "id": "test-foo",
            "text": "Tests assert through the public seam.",
            "source": { "path": "testing/test-foo/README.md" },
            "scope": ["valid/**", "invalid/**"],
            "when": "edit",
            "check": {
                "type": "model",
                "question": { "type": "boolean", "instructions": "Does the test read a private helper?" },
            },
        }])
    );
}

#[test]
fn a_verdict_out_of_its_band_or_range_fails() {
    let dir = workdir("fail", "edit", &report(("flag", 0.6), ("act", 0.85)));

    let output = rules_test(&dir, &["test-foo"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "FAIL test-foo valid/seam.py: flag 0.60, expected `clear`, at most 0.2\n\
         FAIL test-foo invalid/private.py: act 0.85, expected `act`, at least 0.9\n\
         0 of 2 fixtures as expected\n"
    );
}

#[test]
fn a_fixture_that_abide_did_not_judge_fails() {
    let report = r#"{"byFile": [{"file": "valid/seam.py", "verdicts": [
        {"ruleId": "test-foo", "probability": 0.1, "band": "clear"}]},
        {"file": "invalid/private.py", "verdicts": [], "error": "1 of 1 chunks not judged"}]}"#;
    let dir = workdir("unjudged", "edit", report);

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("FAIL test-foo invalid/private.py: abide gave no verdict\n"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn an_expected_json_that_does_not_cover_the_fixtures_fails_before_abide_runs() {
    let dir = workdir("uncovered", "edit", &report(("clear", 0.1), ("act", 0.93)));
    write(
        &dir.join("cat/testing/test-foo/fixtures/invalid/more.py"),
        "x\n",
    );

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "FAIL test-foo: `invalid/more.py` has no verdict in expected.json\n\
         0 of 0 fixtures as expected\n"
    );
    assert!(!dir.join(".botica/judge/test-foo/called.txt").exists());
}

#[test]
fn a_new_run_starts_from_an_empty_repository() {
    let dir = workdir("rerun", "edit", &report(("clear", 0.1), ("act", 0.93)));
    write(&dir.join(".botica/judge/test-foo/stale.py"), "x\n");

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(!dir.join(".botica/judge/test-foo/stale.py").exists());
}

#[test]
fn a_rule_judged_on_a_turn_is_skipped() {
    let dir = workdir("turn", "turn", "{}");

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "skip test-foo: abide audit judges the rules with `when = \"edit\"` only\n\
         0 of 0 fixtures as expected\n"
    );
}

#[test]
fn a_failure_of_abide_fails_the_run() {
    let dir = workdir("broken", "edit", "not json");

    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("audit --json` printed no report"),
        "{}",
        stderr(&output)
    );

    fs::write(dir.join("abide.sh"), "echo 'no key' >&2\nexit 3\n").expect("write abide");
    let output = rules_test(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("audit` failed: no key"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_rule_named_must_be_a_judgment_rule_of_the_catalog() {
    let dir = workdir("named", "edit", "{}");

    for (name, message) in [
        ("no-utils", "`no-utils` is not a judgment rule"),
        ("nope", "`nope` is not a rule of the catalog"),
    ] {
        let output = rules_test(&dir, &[name]);

        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains(message), "{}", stderr(&output));
    }
}
