//! The fingerprint of each finding in the json and sarif output (ADR 0019 decision 5).

mod common;

use common::{stderr, stdout, write};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const KEY: &str = "0123456789abcdef0123456789abcdef";

fn project(name: &str) -> PathBuf {
    common::project_in("fingerprint", name, None)
}

/// `botica lint` in `dir` with `BOTICA_HISTORY_KEY` set to `key`, or removed.
fn lint(dir: &Path, key: Option<&str>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_botica"));
    command
        .arg("lint")
        .args(args)
        .arg("--no-astgrep")
        .current_dir(dir);
    match key {
        Some(key) => command.env("BOTICA_HISTORY_KEY", key),
        None => command.env_remove("BOTICA_HISTORY_KEY"),
    };
    command.output().expect("run botica")
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&stdout(output)).expect("one JSON document")
}

/// The fingerprint of the one finding of `rule` in the json output.
fn fingerprint_of(output: &Output, rule: &str) -> String {
    let document = json(output);
    let found: Vec<&Value> = document["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .filter(|d| d["rule"] == rule)
        .collect();
    assert_eq!(found.len(), 1, "{document}");
    found[0]["fingerprint"]
        .as_str()
        .unwrap_or_else(|| panic!("no fingerprint in {document}"))
        .to_owned()
}

/// A file whose one finding, an unused suppression, is on its second line.
const UNUSED: &str = "import os\nx = 1  # botica: ignore[no-utils]\ny = 2\n";

#[test]
fn a_finding_that_moves_down_keeps_its_fingerprint() {
    let dir = project("moves");
    write(&dir, "a.py", UNUSED);
    let before = fingerprint_of(
        &lint(&dir, None, &["--output-format", "json"]),
        "unused-suppression",
    );

    write(&dir, "a.py", &format!("\"\"\"A module.\"\"\"\n\n{UNUSED}"));
    let after = lint(&dir, None, &["--output-format", "json"]);

    assert_eq!(fingerprint_of(&after, "unused-suppression"), before);
    assert_eq!(before.len(), 16);
    assert!(before.chars().all(|c| c.is_ascii_hexdigit()), "{before}");
}

#[test]
fn an_edit_of_the_line_gives_a_new_fingerprint() {
    let dir = project("edited");
    write(&dir, "a.py", UNUSED);
    let before = fingerprint_of(
        &lint(&dir, None, &["--output-format", "json"]),
        "unused-suppression",
    );

    write(
        &dir,
        "a.py",
        "import os\nx = 2  # botica: ignore[no-utils]\ny = 2\n",
    );
    let after = lint(&dir, None, &["--output-format", "json"]);

    assert_ne!(fingerprint_of(&after, "unused-suppression"), before);
}

#[test]
fn sarif_carries_the_same_fingerprint_as_json() {
    let dir = project("sarif");
    write(&dir, "a.py", UNUSED);

    let from_json = fingerprint_of(
        &lint(&dir, None, &["--output-format", "json"]),
        "unused-suppression",
    );
    let sarif = json(&lint(&dir, None, &["--output-format", "sarif"]));

    let result = &sarif["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "unused-suppression");
    assert_eq!(result["partialFingerprints"]["boticaFinding/v1"], from_json);
}

#[test]
fn the_path_is_taken_from_the_root_of_the_project_whatever_the_working_directory() {
    let dir = project("prefix");
    write(&dir, "botica.toml", "");
    write(&dir, "pkg/a.py", UNUSED);

    let from_root = fingerprint_of(
        &lint(&dir, None, &["pkg/a.py", "--output-format", "json"]),
        "unused-suppression",
    );
    let from_pkg = fingerprint_of(
        &lint(&dir.join("pkg"), None, &["a.py", "--output-format", "json"]),
        "unused-suppression",
    );

    assert_eq!(from_pkg, from_root);
}

#[test]
fn botica_history_key_changes_the_fingerprint() {
    let dir = project("keyed");
    write(&dir, "a.py", UNUSED);

    let unkeyed = lint(&dir, None, &["--output-format", "json"]);
    let keyed = lint(&dir, Some(KEY), &["--output-format", "json"]);
    let empty = lint(&dir, Some(""), &["--output-format", "json"]);

    let unkeyed = fingerprint_of(&unkeyed, "unused-suppression");
    assert_ne!(fingerprint_of(&keyed, "unused-suppression"), unkeyed);
    assert_eq!(fingerprint_of(&empty, "unused-suppression"), unkeyed);
}

#[test]
fn a_short_key_leaves_the_fingerprints_out_with_a_warning_and_the_same_exit_code() {
    let dir = project("short");
    write(&dir, "a.py", UNUSED);

    let output = lint(&dir, Some("short"), &["--output-format", "json"]);

    let document = json(&output);
    assert_eq!(document["diagnostics"][0]["fingerprint"], Value::Null);
    assert!(
        stderr(&output).contains("`BOTICA_HISTORY_KEY` holds 5 bytes and needs at least 32"),
        "{}",
        stderr(&output)
    );
    assert_eq!(
        output.status.code(),
        lint(&dir, None, &["--output-format", "json"]).status.code()
    );
}

#[test]
fn the_text_output_is_unchanged() {
    let dir = project("text");
    write(&dir, "a.py", UNUSED);

    let output = lint(&dir, Some("short"), &[]);

    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
    assert!(!stdout(&output).contains("fingerprint"));
}

#[test]
fn an_extension_process_does_not_inherit_the_key() {
    let dir = project("extension");
    write(&dir, "a.py", "x = 1\n");
    write(
        &dir,
        "ext.sh",
        r#"printf '%s' "${BOTICA_HISTORY_KEY-unset}" > seen.txt
while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"rules":[{"id":"ext-rule","languages":["python"]}]}}' ;;
    *'"lint/files"'*) printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[{"path":"a.py","rule":"ext-rule","severity":"warning","start":0,"end":1,"message":"from ext"}]}}' ;;
    *'"shutdown"'*) printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":null}'; exit 0 ;;
  esac
done
"#,
    );

    let output = lint(
        &dir,
        Some(KEY),
        &["a.py", "--ext", "sh ext.sh", "--output-format", "json"],
    );

    assert_eq!(
        std::fs::read_to_string(dir.join("seen.txt")).expect("the extension ran"),
        "unset"
    );
    assert_eq!(fingerprint_of(&output, "ext-rule").len(), 16);
}

#[test]
fn a_finding_of_a_message_has_the_fingerprint_of_its_message_wherever_it_was_read() {
    let dir = project("message");
    write(
        &dir,
        "botica.toml",
        "preview = true\nselect = [\"conventional-commits\"]\n",
    );
    write(&dir, "msg.txt", "Second commit\n");

    let from_file = lint(
        &dir,
        None,
        &["--commit-msg", "msg.txt", "--output-format", "json"],
    );
    let from_title = lint(
        &dir,
        None,
        &["--pr-title=Second commit", "--output-format", "json"],
    );

    assert_eq!(
        fingerprint_of(&from_title, "conventional-commits"),
        fingerprint_of(&from_file, "conventional-commits")
    );
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?}");
}

/// A judge's finding of `test-through-interface` on line 2 of `tests/test_a.py`.
const JUDGED: &str = r#"{"version": "2.1.0", "runs": [{"results": [{"ruleId": "test-through-interface", "message": {"text": "m"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "tests/test_a.py"}, "region": {"startLine": 2}}}]}]}]}"#;

#[test]
fn a_judged_finding_of_a_staged_run_hashes_the_staged_text_and_not_the_working_tree() {
    let dir = project("judged-staged");
    std::fs::remove_dir_all(dir.join(".git")).expect("remove the stand-in .git");
    git(&dir, &["init", "--quiet"]);
    write(
        &dir,
        "botica.toml",
        "preview = true\nselect = [\"test-through-interface\"]\n",
    );
    write(&dir, "judge.sarif", JUDGED);
    write(&dir, "tests/test_a.py", "def test_a():\n");
    git(&dir, &["add", "--all"]);
    git(&dir, &["commit", "--quiet", "-m", "feat: a test"]);
    write(&dir, "tests/test_a.py", "def test_a():\n    assert a._x\n");
    git(&dir, &["add", "tests/test_a.py"]);
    let args = [
        "--judged",
        "judge.sarif",
        "--staged",
        "--output-format",
        "json",
    ];
    let clean = fingerprint_of(&lint(&dir, None, &args), "test-through-interface");

    write(&dir, "tests/test_a.py", "def test_b():\n    assert a._x\n");
    let dirty = lint(&dir, None, &args);

    assert_eq!(fingerprint_of(&dirty, "test-through-interface"), clean);
}
