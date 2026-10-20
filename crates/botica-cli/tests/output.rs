mod common;

use common::{botica, fake_extension, lines, stdout, write};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Output;

fn project(name: &str, config: Option<&str>) -> PathBuf {
    common::project_in("output", name, config)
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 output")
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&stdout(output)).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON document ({error}):\n{}",
            stdout(output)
        )
    })
}

/// A project with one finding of each kind: whole file, in a line, and an unreadable path.
fn mixed(name: &str) -> PathBuf {
    let dir = project(name, None);
    write(&dir, "big.py", &lines(701));
    write(&dir, "src/a.py", "x = 1  # botica: ignore[no-utils]\n");
    dir
}

#[test]
fn json_output_has_the_schema_version_the_codes_and_a_null_range_for_whole_files() {
    let dir = mixed("json");

    let output = botica(&dir, &[".", "typo/", "--output-format", "json"]);

    let document = json(&output);
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["errors"], serde_json::json!([]));
    let diagnostics = document["diagnostics"].as_array().expect("diagnostics");
    let by_rule = |rule: &str| {
        diagnostics
            .iter()
            .find(|d| d["rule"] == rule)
            .unwrap_or_else(|| panic!("no {rule} in {document}"))
    };
    let big = by_rule("file-size-guard");
    assert_eq!(big["code"], "BTSTRUC001");
    assert_eq!(big["severity"], "error");
    assert_eq!(big["path"], "big.py");
    assert_eq!(big["range"], Value::Null);
    let unused = by_rule("unused-suppression");
    assert_eq!(unused["code"], "BT002");
    assert_eq!(unused["range"]["start"]["line"], 1);
    assert_eq!(unused["range"]["start"]["column"], 10);
    assert_eq!(unused["range"]["end"]["line"], 1);
    assert!(
        unused["range"]["end"]["offset"].as_u64() > unused["range"]["start"]["offset"].as_u64()
    );
    assert_eq!(by_rule("io-error")["path"], "typo/");
    assert_eq!(big["standard"], "CES-71");
    assert_eq!(unused["standard"], Value::Null);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn json_says_the_same_as_the_text_output() {
    let dir = mixed("json-text");
    let args = [".", "typo/"];

    let text = stdout(&botica(&dir, &args));
    let with_json = botica(&dir, &[".", "typo/", "--output-format", "json"]);

    let rebuilt: Vec<String> = json(&with_json)["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|d| {
            let (line, column) = match d["range"].is_null() {
                true => (1, 1),
                false => (
                    d["range"]["start"]["line"].as_u64().expect("line"),
                    d["range"]["start"]["column"].as_u64().expect("column"),
                ),
            };
            format!(
                "{}:{line}:{column}: {} [{}] {}",
                d["path"].as_str().expect("path"),
                d["severity"].as_str().expect("severity"),
                d["rule"].as_str().expect("rule"),
                d["message"].as_str().expect("message"),
            )
        })
        .collect();
    assert_eq!(rebuilt, text.lines().collect::<Vec<_>>());
    assert!(!rebuilt.is_empty());
}

#[test]
fn sarif_output_is_a_2_1_0_document_with_the_catalog_rules_and_one_result_per_diagnostic() {
    let dir = mixed("sarif");

    let output = botica(&dir, &[".", "--output-format", "sarif"]);

    let document = json(&output);
    assert_eq!(document["version"], "2.1.0");
    let run = &document["runs"][0];
    let ids: Vec<&str> = run["tool"]["driver"]["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .map(|rule| rule["id"].as_str().expect("id"))
        .collect();
    for rule in [
        "io-error",
        "unused-suppression",
        "file-size-guard",
        "no-utils",
    ] {
        assert!(ids.contains(&rule), "{rule} missing from {ids:?}");
    }
    let results = run["results"].as_array().expect("results");
    assert_eq!(results.len(), 2, "{document}");
    for result in results {
        let index = result["ruleIndex"].as_u64().expect("index") as usize;
        assert_eq!(ids[index], result["ruleId"]);
    }
    assert_eq!(run["invocations"][0]["executionSuccessful"], true);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn github_output_is_one_annotation_per_diagnostic() {
    let dir = mixed("github");

    let output = botica(&dir, &[".", "--output-format", "github"]);

    let text = stdout(&output);
    assert!(
        text.contains("::error file=big.py,title=botica (file-size-guard)::"),
        "{text}"
    );
    assert!(
        text.contains("::warning file=src/a.py,line=1,col=10,endLine=1,endColumn="),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn an_unknown_output_format_is_a_usage_error() {
    let dir = project("bad-format", None);

    let output = botica(&dir, &[".", "--output-format", "xml"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("invalid value 'xml'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn exit_zero_forces_success_when_only_violations_are_left() {
    let dir = mixed("exit-zero");

    let output = botica(&dir, &[".", "--exit-zero"]);

    assert!(stdout(&output).contains("[file-size-guard]"));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn fail_on_names_the_lowest_severity_that_fails_the_run() {
    let dir = project("fail-on", None);
    write(&dir, "src/a.py", "x = 1  # botica: ignore[no-utils]\n");

    let code = |args: &[&str]| botica(&dir, args).status.code();

    assert_eq!(code(&["."]), Some(0), "a warning alone does not fail");
    assert_eq!(code(&[".", "--fail-on", "warning"]), Some(1));
    assert_eq!(code(&[".", "--fail-on", "info"]), Some(1));
    assert_eq!(code(&[".", "--fail-on", "warning", "--exit-zero"]), Some(0));
}

#[test]
fn a_clean_run_exits_zero_and_json_is_still_a_document() {
    let dir = project("clean", None);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &[".", "--output-format", "json"]);

    assert_eq!(json(&output)["diagnostics"], serde_json::json!([]));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn the_diagnostics_of_an_extension_are_ordered_with_the_rest() {
    let dir = project("ext-order", None);
    write(&dir, "big.py", &lines(701));
    write(&dir, "a.py", "x = 1\n");
    write(&dir, "b.py", "x = 1\n");
    fake_extension(
        &dir,
        r#"{"path":"b.py","rule":"ext-rule","severity":"warning","start":0,"end":1,"message":"from ext"},{"path":"a.py","rule":"ext-rule","severity":"warning","start":2,"end":3,"message":"from ext"}"#,
    );

    let output = botica(&dir, &["a.py", "b.py", "big.py", "--ext", "sh ext.sh"]);

    let text = stdout(&output);
    let order: Vec<&str> = text
        .lines()
        .map(|line| line.split(':').next().unwrap_or(""))
        .collect();
    assert_eq!(order, ["a.py", "b.py", "big.py"], "{text}");
    assert!(
        text.contains("a.py:1:3: warning [ext-rule] from ext"),
        "{text}"
    );
}

#[test]
fn a_failing_extension_is_an_error_of_the_run_and_the_rest_is_still_reported() {
    let dir = project("ext-fails", None);
    write(&dir, "big.py", &lines(701));
    write(&dir, "ext.sh", "read -r line\nexit 0\n");

    let output = botica(
        &dir,
        &[
            ".",
            "--ext",
            "sh ext.sh",
            "--output-format",
            "json",
            "--exit-zero",
        ],
    );

    let document = json(&output);
    let errors = document["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{document}");
    assert_eq!(errors[0]["kind"], "extension");
    assert_eq!(errors[0]["extension"], "sh ext.sh");
    assert!(errors[0]["message"]
        .as_str()
        .is_some_and(|m| m.contains("closed its stdout")));
    assert_eq!(document["diagnostics"][0]["rule"], "file-size-guard");
    assert_eq!(
        output.status.code(),
        Some(2),
        "an operational failure wins over --exit-zero"
    );
}

#[test]
fn in_text_a_failing_extension_is_said_on_stderr() {
    let dir = project("ext-fails-text", None);
    write(&dir, "big.py", &lines(701));
    write(&dir, "ext.sh", "read -r line\nexit 0\n");

    let output = botica(&dir, &[".", "--ext", "sh ext.sh"]);

    assert!(stdout(&output).contains("[file-size-guard]"));
    assert!(
        stderr(&output).contains("extension `sh ext.sh`"),
        "{}",
        stderr(&output)
    );
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn an_extension_that_does_not_start_is_the_same_kind_of_error() {
    let dir = project("ext-missing", None);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(
        &dir,
        &[
            ".",
            "--ext",
            "no-such-extension-binary",
            "--output-format",
            "sarif",
        ],
    );

    let document = json(&output);
    assert_eq!(
        document["runs"][0]["invocations"][0]["executionSuccessful"],
        false
    );
    assert_eq!(output.status.code(), Some(2));
}
