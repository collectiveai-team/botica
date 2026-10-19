//! The findings of a judge (`botica lint --judged <sarif>`): botica reads them from SARIF and
//! filters them with the configuration and the suppression comments of the project, as it
//! filters the findings of its own rules.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use botica_config::Resolution;
use botica_core::{Diagnostic, Severity};
use serde::Deserialize;

use crate::project;

#[derive(Deserialize)]
struct Sarif {
    runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    #[serde(default)]
    results: Vec<SarifResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    message: Message,
    #[serde(default)]
    locations: Vec<Location>,
}

#[derive(Deserialize)]
struct Message {
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: Physical,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Physical {
    artifact_location: Artifact,
    region: Option<Region>,
}

#[derive(Deserialize)]
struct Artifact {
    uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: usize,
    start_column: Option<usize>,
    end_line: Option<usize>,
    end_column: Option<usize>,
}

/// What botica keeps of a judge's findings, and the `ruleId`s it left out because they name no
/// judgment rule of the catalog.
pub struct Judged {
    pub diagnostics: Vec<Diagnostic>,
    pub left_out: Vec<String>,
}

/// A URI reference of SARIF as a relative path: `%XX` decoded.
fn path_of(uri: &str) -> String {
    let bytes = uri.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| uri.get(i + 1..i + 3))
            .flatten()
            .and_then(|digits| u8::from_str_radix(digits, 16).ok());
        match hex {
            Some(byte) => {
                decoded.push(byte);
                i += 3;
            }
            None => {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn diagnostic(result: SarifResult, rule: &str, severity: Severity) -> Diagnostic {
    let physical = result
        .locations
        .into_iter()
        .next()
        .map(|l| l.physical_location);
    let path = physical
        .as_ref()
        .map_or_else(String::new, |p| path_of(&p.artifact_location.uri));
    let region = physical.and_then(|p| p.region);
    let (line, column) = region
        .as_ref()
        .map_or((1, 1), |r| (r.start_line, r.start_column.unwrap_or(1)));
    Diagnostic {
        path,
        rule: rule.to_owned(),
        severity,
        start: 0,
        end: 0,
        line,
        column,
        end_line: region.as_ref().and_then(|r| r.end_line).unwrap_or(line),
        end_column: region.as_ref().and_then(|r| r.end_column).unwrap_or(column),
        file_level: region.is_none(),
        message: result.message.text,
    }
}

/// Reads the SARIF of a judge and keeps what the project reports: the findings of its active
/// judgment rules, at the paths and in the languages they apply to, with their severity, minus
/// what an `ignore-file` comment of the file silences. `read` gives the text of a file.
pub fn filter(
    resolution: &Arc<Resolution>,
    prefix: Option<&str>,
    sarif: &str,
    read: impl Fn(&str) -> Option<String>,
) -> Result<Judged> {
    let sarif: Sarif = serde_json::from_str(sarif).context("the judged findings are not SARIF")?;
    let mut by_path: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    let mut change = Vec::new();
    let mut left_out = Vec::new();
    for result in sarif.runs.into_iter().flat_map(|run| run.results) {
        let Some(entry) = resolution
            .entry(&result.rule_id)
            .filter(|entry| entry.rule.is_judgment())
        else {
            left_out.push(result.rule_id);
            continue;
        };
        let found = diagnostic(result, &entry.rule.id, Severity::from(entry.severity));
        if found.path.is_empty() {
            if entry.active {
                change.push(found);
            }
        } else {
            by_path.entry(found.path.clone()).or_default().push(found);
        }
    }
    let mut diagnostics = project::keep_judged(resolution, prefix, by_path, read);
    change.extend(diagnostics);
    diagnostics = change;
    Ok(Judged {
        diagnostics,
        left_out,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::*;
    use botica_catalog::{read_files, Catalog, SourceFile};
    use botica_config::{Config, Resolution};

    const RULE: &str = "id = \"test-foo\"\ncode = \"BTTEST001\"\nsummary = \"s\"\n\
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
        let judged = judge("", &[result("BTTEST001", &at("src/a.py", 3))], &[]);

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
}
