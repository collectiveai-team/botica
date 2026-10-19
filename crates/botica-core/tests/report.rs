use botica_core::report::{write, ErrorKind, Format, Report, RuleDoc, RunError};
use botica_core::{Diagnostic, Severity};
use serde_json::{json, Value};

fn in_code(path: &str, rule: &str, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: rule.to_owned(),
        severity,
        start: 10,
        end: 18,
        line: 2,
        column: 1,
        end_line: 2,
        end_column: 9,
        file_level: false,
        message: message.to_owned(),
    }
}

fn whole_file(path: &str, rule: &str, severity: Severity, message: &str) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: rule.to_owned(),
        severity,
        start: 0,
        end: 0,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
        file_level: true,
        message: message.to_owned(),
    }
}

fn catalog() -> Vec<RuleDoc> {
    vec![RuleDoc {
        id: "file-size-guard".to_owned(),
        code: "BTSTRUC001".to_owned(),
        summary: "Files that grow too large".to_owned(),
        severity: Severity::Error,
    }]
}

fn extension_failure() -> RunError {
    RunError {
        kind: ErrorKind::Extension,
        extension: Some("uv run --script ext.py".to_owned()),
        path: None,
        message: "extension closed its stdout during `lint/files`".to_owned(),
    }
}

fn rendered(diagnostics: &[Diagnostic], errors: &[RunError], format: Format) -> String {
    let rules = catalog();
    let report = Report {
        diagnostics,
        errors,
        rules: &rules,
        tool_version: "1.2.3",
    };
    let mut out = Vec::new();
    write(&report, format, &mut out).expect("write to memory");
    String::from_utf8(out).expect("UTF-8 output")
}

#[test]
fn text_is_one_line_per_diagnostic_and_leaves_the_errors_to_the_caller() {
    let diagnostics = [
        in_code("src/a.py", "log-no-print", Severity::Warning, "no print"),
        whole_file("big.py", "file-size-guard", Severity::Error, "too large"),
    ];

    let text = rendered(&diagnostics, &[extension_failure()], Format::Text);

    assert_eq!(
        text,
        "src/a.py:2:1: warning [log-no-print] no print\nbig.py:1:1: error [file-size-guard] too large\n"
    );
}

#[test]
fn github_annotates_each_diagnostic_and_each_error_escaping_what_the_workflow_commands_reserve() {
    let diagnostics = [
        in_code("src/a.py", "log-no-print", Severity::Warning, "no print"),
        in_code(
            "src/b.py",
            "note-rule",
            Severity::Info,
            "50% done\nsecond, line\r",
        ),
        in_code("src/c.py", "hint-rule", Severity::Hint, "a hint"),
        whole_file(
            "odd,name:x.py",
            "file-size-guard",
            Severity::Error,
            "too large",
        ),
    ];

    let text = rendered(&diagnostics, &[extension_failure()], Format::Github);

    assert_eq!(
        text,
        concat!(
            "::warning file=src/a.py,line=2,col=1,endLine=2,endColumn=9,title=botica (log-no-print)::no print\n",
            "::notice file=src/b.py,line=2,col=1,endLine=2,endColumn=9,title=botica (note-rule)::50%25 done%0Asecond, line%0D\n",
            "::notice file=src/c.py,line=2,col=1,endLine=2,endColumn=9,title=botica (hint-rule)::a hint\n",
            "::error file=odd%2Cname%3Ax.py,title=botica (file-size-guard)::too large\n",
            "::error title=botica::extension `uv run --script ext.py`: extension closed its stdout during `lint/files`\n",
        )
    );
}

#[test]
fn sarif_lists_the_rules_locates_each_result_and_reports_the_errors_as_notifications() {
    let diagnostics = [
        in_code(
            "src/my file.py",
            "log-no-print",
            Severity::Warning,
            "no print",
        ),
        whole_file("big.py", "file-size-guard", Severity::Error, "too large"),
    ];

    let text = rendered(&diagnostics, &[extension_failure()], Format::Sarif);

    let document: Value = serde_json::from_str(&text).expect("one JSON document");
    assert_eq!(
        document,
        json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {"driver": {
                    "name": "botica",
                    "version": "1.2.3",
                    "rules": [
                        {
                            "id": "file-size-guard",
                            "shortDescription": {"text": "Files that grow too large"},
                            "defaultConfiguration": {"level": "error"},
                            "properties": {"code": "BTSTRUC001"}
                        },
                        {"id": "log-no-print"}
                    ]
                }},
                "columnKind": "unicodeCodePoints",
                "results": [
                    {
                        "ruleId": "log-no-print",
                        "ruleIndex": 1,
                        "level": "warning",
                        "message": {"text": "no print"},
                        "locations": [{"physicalLocation": {
                            "artifactLocation": {"uri": "src/my%20file.py"},
                            "region": {
                                "startLine": 2, "startColumn": 1,
                                "endLine": 2, "endColumn": 9,
                                "byteOffset": 10, "byteLength": 8
                            }
                        }}]
                    },
                    {
                        "ruleId": "file-size-guard",
                        "ruleIndex": 0,
                        "level": "error",
                        "message": {"text": "too large"},
                        "locations": [{"physicalLocation": {
                            "artifactLocation": {"uri": "big.py"}
                        }}]
                    }
                ],
                "invocations": [{
                    "executionSuccessful": false,
                    "toolExecutionNotifications": [{
                        "level": "error",
                        "message": {"text": "extension `uv run --script ext.py`: extension closed its stdout during `lint/files`"}
                    }]
                }]
            }]
        })
    );
}

#[test]
fn sarif_of_a_clean_run_is_successful_and_has_no_results() {
    let text = rendered(&[], &[], Format::Sarif);

    let document: Value = serde_json::from_str(&text).expect("one JSON document");
    let run = &document["runs"][0];
    assert_eq!(run["results"], json!([]));
    assert_eq!(run["invocations"], json!([{"executionSuccessful": true}]));
}

#[test]
fn sarif_levels_follow_the_severity() {
    let diagnostics = [
        in_code("a.py", "r", Severity::Error, ""),
        in_code("a.py", "r", Severity::Warning, ""),
        in_code("a.py", "r", Severity::Info, ""),
        in_code("a.py", "r", Severity::Hint, ""),
    ];

    let text = rendered(&diagnostics, &[], Format::Sarif);

    let document: Value = serde_json::from_str(&text).expect("one JSON document");
    let levels: Vec<&str> = document["runs"][0]["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|result| result["level"].as_str().expect("level"))
        .collect();
    assert_eq!(levels, ["error", "warning", "note", "note"]);
}

fn read_failure() -> RunError {
    RunError {
        kind: ErrorKind::Read,
        extension: None,
        path: Some("data/locked.txt".to_owned()),
        message: "Permission denied (os error 13)".to_owned(),
    }
}

#[test]
fn json_names_the_file_of_a_read_error_and_not_an_extension() {
    let text = rendered(&[], &[read_failure()], Format::Json);

    let document: Value = serde_json::from_str(&text).expect("one JSON document");
    assert_eq!(
        document["errors"],
        json!([
            {
                "kind": "read",
                "path": "data/locked.txt",
                "message": "Permission denied (os error 13)"
            }
        ])
    );
}

#[test]
fn github_says_a_read_error_with_the_file_in_the_message() {
    let text = rendered(&[], &[read_failure()], Format::Github);

    assert_eq!(
        text,
        "::error title=botica::read `data/locked.txt`: Permission denied (os error 13)\n"
    );
}

#[test]
fn json_carries_the_schema_version_the_diagnostics_and_the_errors_apart() {
    let diagnostics = [
        in_code("src/a.py", "log-no-print", Severity::Warning, "no print"),
        whole_file("big.py", "file-size-guard", Severity::Error, "too large"),
    ];

    let text = rendered(&diagnostics, &[extension_failure()], Format::Json);

    let document: Value = serde_json::from_str(&text).expect("one JSON document");
    assert_eq!(
        document,
        json!({
            "schema_version": 1,
            "diagnostics": [
                {
                    "rule": "log-no-print",
                    "code": null,
                    "severity": "warning",
                    "path": "src/a.py",
                    "range": {
                        "start": {"offset": 10, "line": 2, "column": 1},
                        "end": {"offset": 18, "line": 2, "column": 9}
                    },
                    "message": "no print"
                },
                {
                    "rule": "file-size-guard",
                    "code": "BTSTRUC001",
                    "severity": "error",
                    "path": "big.py",
                    "range": null,
                    "message": "too large"
                }
            ],
            "errors": [
                {
                    "kind": "extension",
                    "extension": "uv run --script ext.py",
                    "message": "extension closed its stdout during `lint/files`"
                }
            ]
        })
    );
}

#[test]
fn text_shows_a_finding_about_a_whole_change_as_change() {
    let change = whole_file(
        "",
        "test-through-interface",
        Severity::Warning,
        "a test reaches into a private helper",
    );

    assert_eq!(
        rendered(&[change], &[], Format::Text),
        "(change): warning [test-through-interface] a test reaches into a private helper\n"
    );
}
