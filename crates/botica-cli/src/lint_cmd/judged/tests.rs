use std::path::Path;
use std::sync::Arc;

use super::*;
use botica_catalog::{read_files, Catalog, SourceFile};
use botica_config::{Config, Resolution};

const RULE: &str = "id = \"test-foo\"\ncode = \"BTTEST901\"\nsummary = \"s\"\n\
    status = \"stable\"\ndefault-enabled = true\ndefault-severity = \"warning\"\n\n\
    [[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\n\
    why-not-deterministic = \"w\"\n\n\
    [implementations.question]\ntype = \"boolean\"\ninstructions = \"q\"\n";

const README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
    ## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
    ## Suppression\n\ns\n";

/// The catalog of this checkout with one judgment rule, `test-foo`, under `config`.
fn resolution(config: &str) -> Arc<Resolution> {
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/rules");
    let mut files = read_files(&rules).expect("the catalog is read");
    for (name, text) in [("rule.toml", RULE), ("README.md", README)] {
        files.push(SourceFile {
            path: format!("testing/test-foo/{name}"),
            text: text.to_owned(),
        });
    }
    let catalog = Catalog::from_files(&files).unwrap_or_else(|error| panic!("{error}"));
    let config = Config::from_botica_toml(config).expect("a valid configuration");
    Arc::new(Resolution::new(&config, &catalog).expect("it resolves"))
}

fn result(rule: &str, location: &str) -> String {
    format!(
        "{{\"ruleId\": \"{rule}\", \"message\": {{\"text\": \"the test reaches into a private \
         helper\"}}, \"locations\": [{location}]}}"
    )
}

fn at(uri: &str, line: usize) -> String {
    format!(
        "{{\"physicalLocation\": {{\"artifactLocation\": {{\"uri\": \"{uri}\"}}, \
         \"region\": {{\"startLine\": {line}, \"startColumn\": 5, \"endLine\": {line}, \
         \"endColumn\": 9}}}}}}"
    )
}

fn sarif(results: &[String]) -> String {
    format!(
        "{{\"version\": \"2.1.0\", \"runs\": [{{\"results\": [{}]}}]}}",
        results.join(",")
    )
}

fn judge(config: &str, results: &[String], files: &[(&str, &str)]) -> Judged {
    let read = |path: &str| {
        files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, text)| (*text).to_owned())
    };
    filter(&resolution(config), None, &sarif(results), read).expect("the SARIF is read")
}

fn summary(judged: &Judged) -> Vec<String> {
    judged
        .diagnostics
        .iter()
        .map(|d| format!("{}@{}:{} {}", d.rule, d.path, d.line, d.severity.as_str()))
        .collect()
}

#[test]
fn a_finding_of_an_active_judgment_rule_is_reported_where_the_judge_puts_it() {
    let judged = judge("", &[result("BTTEST901", &at("src/a.py", 3))], &[]);

    let finding = &judged.diagnostics[0];
    assert_eq!(summary(&judged), ["test-foo@src/a.py:3 warning"]);
    assert_eq!(
        (finding.column, finding.end_line, finding.end_column),
        (5, 3, 9)
    );
    assert!(!finding.file_level);
    assert_eq!(finding.message, "the test reaches into a private helper");
    assert!(judged.left_out.is_empty());
}

#[test]
fn the_uri_of_a_finding_is_decoded_into_its_path() {
    let judged = judge("", &[result("test-foo", &at("src/my%20app/a.py", 1))], &[]);

    assert_eq!(summary(&judged), ["test-foo@src/my app/a.py:1 warning"]);
}

#[test]
fn a_finding_about_a_whole_change_has_no_path_and_no_range() {
    let judged = judge("", &[result("test-foo", "")], &[]);

    assert_eq!(summary(&judged), ["test-foo@:1 warning"]);
    assert!(judged.diagnostics[0].file_level);
}

#[test]
fn an_ignore_file_comment_of_the_file_silences_the_finding() {
    let judged = judge(
        "",
        &[
            result("test-foo", &at("src/a.py", 3)),
            result("test-foo", &at("src/b.py", 3)),
        ],
        &[(
            "src/a.py",
            "# botica: ignore-file[test-foo] -- legacy importer\n",
        )],
    );

    assert_eq!(summary(&judged), ["test-foo@src/b.py:3 warning"]);
}

#[test]
fn the_configuration_drops_the_rule_its_paths_and_sets_its_severity() {
    let both = [
        result("test-foo", &at("legacy/a.py", 1)),
        result("test-foo", &at("src/a.py", 1)),
        result("test-foo", ""),
    ];

    assert!(judge("ignore = [\"test-foo\"]\n", &both, &[])
        .diagnostics
        .is_empty());
    assert_eq!(
        summary(&judge(
            "[rules.test-foo]\nseverity = \"error\"\nextend-ignores = [\"legacy/**\"]\n",
            &both,
            &[]
        )),
        ["test-foo@:1 error", "test-foo@src/a.py:1 error"]
    );
}

#[test]
fn a_file_in_a_language_the_rule_does_not_judge_is_left_out() {
    assert!(judge("", &[result("test-foo", &at("web/a.ts", 1))], &[])
        .diagnostics
        .is_empty());
}

#[test]
fn a_result_of_a_rule_that_is_not_judgment_is_left_out_and_named() {
    let judged = judge(
        "",
        &[
            result("no-utils", &at("src/utils.py", 1)),
            result("nope", ""),
        ],
        &[],
    );

    assert!(judged.diagnostics.is_empty());
    assert_eq!(judged.left_out, ["no-utils", "nope"]);
}

#[test]
fn a_sarif_that_is_not_json_is_an_error() {
    let read = |_: &str| None;
    assert!(filter(&resolution(""), None, "not json", read).is_err());
}

#[test]
fn a_document_that_is_neither_sarif_nor_a_report_of_abide_is_an_error() {
    let read = |_: &str| None;
    let error = filter(&resolution(""), None, "{\"version\": 1}", read)
        .err()
        .expect("it is refused");
    assert!(error.to_string().contains("neither SARIF nor"), "{error}");
}

fn verdict(rule: &str, band: &str, extra: &str) -> String {
    format!("{{\"ruleId\": \"{rule}\", \"probability\": 0.9, \"band\": \"{band}\"{extra}}}")
}

fn section(phase: &str, files: &[&str], verdicts: &[String]) -> String {
    let files: Vec<String> = files.iter().map(|f| format!("\"{f}\"")).collect();
    format!(
        "{{\"phase\": \"{phase}\", \"files\": [{}], \"modelRules\": 1, \"calls\": 1, \
         \"latencyMs\": 10, \"verdicts\": [{}]}}",
        files.join(","),
        verdicts.join(",")
    )
}

fn abide(config: &str, sections: &[String], files: &[(&str, &str)]) -> Judged {
    let report = format!(
        "{{\"root\": \"/repo\", \"sections\": [{}], \"spendUsd\": 0.001, \"all\": false}}",
        sections.join(",")
    );
    let read = |path: &str| {
        files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, text)| (*text).to_owned())
    };
    filter(&resolution(config), None, &report, read).expect("the report is read")
}

#[test]
fn an_act_verdict_of_abide_is_a_finding_about_the_file_of_its_section() {
    let judged = abide(
        "",
        &[section(
            "edit",
            &["src/a.py"],
            &[verdict("test-foo", "act", ", \"answer\": \"violating\"")],
        )],
        &[],
    );

    assert_eq!(summary(&judged), ["test-foo@src/a.py:1 warning"]);
    let finding = &judged.diagnostics[0];
    assert!(finding.file_level);
    assert_eq!(
        finding.message,
        "s (abide sees the change break it: act, probability 0.90, answer violating)"
    );
}

#[test]
fn a_flag_verdict_is_info_whatever_the_severity_and_a_clear_one_is_no_finding() {
    let judged = abide(
        "[rules.test-foo]\nseverity = \"error\"\n",
        &[
            section("edit", &["src/a.py"], &[verdict("test-foo", "flag", "")]),
            section("edit", &["src/b.py"], &[verdict("test-foo", "clear", "")]),
            section("edit", &["src/c.py"], &[verdict("test-foo", "act", "")]),
        ],
        &[],
    );

    assert_eq!(
        summary(&judged),
        ["test-foo@src/c.py:1 error", "test-foo@src/a.py:1 info"]
    );
}

#[test]
fn a_verdict_of_the_turn_phase_is_about_its_file_or_about_the_whole_change() {
    let judged = abide(
        "",
        &[section(
            "turn",
            &["src/a.py", "src/b.py"],
            &[
                verdict("test-foo", "act", ""),
                verdict("test-foo", "act", ", \"file\": \"src/b.py\""),
            ],
        )],
        &[],
    );

    assert_eq!(
        summary(&judged),
        ["test-foo@:1 warning", "test-foo@src/b.py:1 warning"]
    );
}

#[test]
fn an_ignore_file_comment_silences_a_verdict_of_abide() {
    let judged = abide(
        "",
        &[
            section("edit", &["src/a.py"], &[verdict("test-foo", "act", "")]),
            section("edit", &["src/b.py"], &[verdict("test-foo", "flag", "")]),
        ],
        &[
            (
                "src/a.py",
                "# botica: ignore-file[test-foo] -- legacy importer\n",
            ),
            (
                "src/b.py",
                "# botica: ignore-file[test-foo] -- legacy importer\n",
            ),
        ],
    );

    assert!(judged.diagnostics.is_empty());
}
