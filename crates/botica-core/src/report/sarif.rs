//! SARIF 2.1.0, the subset GitHub code scanning and other viewers read.

use super::Report;
use crate::diagnostic::Severity;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::io::{self, Write};

#[derive(Serialize)]
struct Document<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: [Run<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Run<'a> {
    tool: Tool<'a>,
    column_kind: &'static str,
    results: Vec<SarifResult<'a>>,
    invocations: [Invocation; 1],
}

#[derive(Serialize)]
struct Tool<'a> {
    driver: Driver<'a>,
}

#[derive(Serialize)]
struct Driver<'a> {
    name: &'static str,
    version: &'a str,
    rules: Vec<Descriptor<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor<'a> {
    id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_description: Option<Text<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_configuration: Option<Configuration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Properties<'a>>,
}

#[derive(Serialize)]
struct Configuration {
    level: &'static str,
}

#[derive(Serialize)]
struct Properties<'a> {
    code: &'a str,
}

#[derive(Serialize)]
struct Text<'a> {
    text: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult<'a> {
    rule_id: &'a str,
    rule_index: usize,
    level: &'static str,
    message: Text<'a>,
    locations: [Location; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Location {
    physical_location: Physical,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Physical {
    artifact_location: Artifact,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Serialize)]
struct Artifact {
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    byte_offset: usize,
    byte_length: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Invocation {
    execution_successful: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_execution_notifications: Vec<Notification>,
}

#[derive(Serialize)]
struct Notification {
    level: &'static str,
    message: OwnedText,
}

#[derive(Serialize)]
struct OwnedText {
    text: String,
}

fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info | Severity::Hint => "note",
    }
}

/// A relative path as a URI reference: everything but the unreserved characters and `/` is
/// percent-encoded, byte by byte.
fn uri(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

/// The catalog's rules first, as given, then the ones that only appear in the results (ast-grep's,
/// an extension's), so every `ruleIndex` points at a descriptor.
fn descriptors<'a>(report: &'a Report<'a>) -> (Vec<Descriptor<'a>>, HashMap<&'a str, usize>) {
    let mut descriptors: Vec<Descriptor<'a>> = report
        .rules
        .iter()
        .map(|rule| Descriptor {
            id: &rule.id,
            short_description: Some(Text {
                text: &rule.summary,
            }),
            default_configuration: Some(Configuration {
                level: level(rule.severity),
            }),
            properties: Some(Properties { code: &rule.code }),
        })
        .collect();
    let known: BTreeSet<&str> = report.rules.iter().map(|rule| rule.id.as_str()).collect();
    let unlisted: BTreeSet<&str> = report
        .diagnostics
        .iter()
        .map(|d| d.rule.as_str())
        .filter(|id| !known.contains(id))
        .collect();
    descriptors.extend(unlisted.into_iter().map(|id| Descriptor {
        id,
        short_description: None,
        default_configuration: None,
        properties: None,
    }));
    let index = descriptors
        .iter()
        .enumerate()
        .map(|(position, descriptor)| (descriptor.id, position))
        .collect();
    (descriptors, index)
}

pub(super) fn write(report: &Report<'_>, out: &mut impl Write) -> io::Result<()> {
    let (rules, index) = descriptors(report);
    let results = report
        .diagnostics
        .iter()
        .map(|d| SarifResult {
            rule_id: &d.rule,
            rule_index: index[d.rule.as_str()],
            level: level(d.severity),
            message: Text { text: &d.message },
            locations: [Location {
                physical_location: Physical {
                    artifact_location: Artifact { uri: uri(&d.path) },
                    region: (!d.file_level).then_some(Region {
                        start_line: d.line,
                        start_column: d.column,
                        end_line: d.end_line,
                        end_column: d.end_column,
                        byte_offset: d.start,
                        byte_length: d.end - d.start,
                    }),
                },
            }],
        })
        .collect();
    let document = Document {
        schema: "https://json.schemastore.org/sarif-2.1.0.json",
        version: "2.1.0",
        runs: [Run {
            tool: Tool {
                driver: Driver {
                    name: "botica",
                    version: report.tool_version,
                    rules,
                },
            },
            column_kind: "unicodeCodePoints",
            results,
            invocations: [Invocation {
                execution_successful: report.errors.is_empty(),
                tool_execution_notifications: report
                    .errors
                    .iter()
                    .map(|error| Notification {
                        level: "error",
                        message: OwnedText {
                            text: error.to_string(),
                        },
                    })
                    .collect(),
            }],
        }],
    };
    serde_json::to_writer(&mut *out, &document)?;
    writeln!(out)
}
