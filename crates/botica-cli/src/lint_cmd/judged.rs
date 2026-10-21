//! The findings of a judge (`botica lint --judged <file>`): botica reads them from SARIF or from
//! the report of `abide check --json`, and filters them with the configuration and the
//! suppression comments of the project, as it filters the findings of its own rules.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use botica_catalog::Probability;
use botica_config::Resolution;
use botica_core::{Diagnostic, Severity};
use serde::Deserialize;
use serde_json::Value;

use crate::project;

mod abide;

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

/// A finding as the judge wrote it, before botica looks its rule up.
struct Found {
    rule_id: String,
    /// The text of the judge, or `None` for the summary of the rule.
    message: Option<String>,
    /// What the judge adds to the message, in parentheses.
    note: Option<String>,
    /// The relative path, empty for a finding about a whole change.
    path: String,
    region: Option<Region>,
    /// A finding the judge is unsure of: it is reported as info, whatever the rule's severity.
    unsure: bool,
    /// The probability the judge gives the violation, when it gives one.
    probability: Option<f64>,
}

impl Found {
    /// A sure finding whose probability is under the `act-at` of its rule becomes one the judge is
    /// unsure of.
    fn under(mut self, act_at: Option<Probability>) -> Self {
        if let (false, Some(probability), Some(Probability(act_at))) =
            (self.unsure, self.probability, act_at)
        {
            if probability < act_at {
                self.unsure = true;
                self.note = Some(format!(
                    "abide sees the change break it with probability {probability:.2}, under the \
                     {act_at:.2} that the rule reports at"
                ));
            }
        }
        self
    }
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

fn from_sarif(document: Value) -> Result<Vec<Found>> {
    let sarif: Sarif =
        serde_json::from_value(document).context("the judged findings are not valid SARIF")?;
    Ok(sarif
        .runs
        .into_iter()
        .flat_map(|run| run.results)
        .map(|result| {
            let physical = result
                .locations
                .into_iter()
                .next()
                .map(|l| l.physical_location);
            Found {
                rule_id: result.rule_id,
                message: Some(result.message.text),
                note: None,
                path: physical
                    .as_ref()
                    .map_or_else(String::new, |p| path_of(&p.artifact_location.uri)),
                region: physical.and_then(|p| p.region),
                unsure: false,
                probability: None,
            }
        })
        .collect())
}

/// The findings of the file of `--judged`: SARIF when it has `runs`, the report of
/// `abide check --json` when it has `sections`.
fn read_findings(text: &str) -> Result<Vec<Found>> {
    let document: Value = serde_json::from_str(text)
        .context("the judged findings are neither SARIF nor a report of `abide check --json`")?;
    if document.get("runs").is_some() {
        from_sarif(document)
    } else if document.get("sections").is_some() {
        abide::findings(document)
    } else {
        bail!("the judged findings are neither SARIF nor a report of `abide check --json`")
    }
}

fn diagnostic(found: Found, summary: &str, rule: &str, severity: Severity) -> Diagnostic {
    let region = found.region;
    let (line, column) = region
        .as_ref()
        .map_or((1, 1), |r| (r.start_line, r.start_column.unwrap_or(1)));
    let text = found.message.unwrap_or_else(|| summary.to_owned());
    Diagnostic {
        path: found.path,
        rule: rule.to_owned(),
        severity,
        start: 0,
        end: 0,
        line,
        column,
        end_line: region.as_ref().and_then(|r| r.end_line).unwrap_or(line),
        end_column: region.as_ref().and_then(|r| r.end_column).unwrap_or(column),
        file_level: region.is_none(),
        measure: None,
        message: match found.note {
            Some(note) => format!("{text} ({note})"),
            None => text,
        },
    }
}

/// The findings of one kind, sure or unsure: those of the active judgment rules about a whole
/// change, then those that the project keeps at their path.
struct Kept {
    change: Vec<Diagnostic>,
    by_path: BTreeMap<String, Vec<Diagnostic>>,
}

/// Reads the findings of a judge and keeps what the project reports: the findings of its active
/// judgment rules, at the paths and in the languages they apply to, with their severity (info for
/// a finding the judge is unsure of), minus what an `ignore-file` comment of the file silences.
/// `read` gives the text of a file.
pub fn filter(
    resolution: &Arc<Resolution>,
    prefix: Option<&str>,
    text: &str,
    read: impl Fn(&str) -> Option<String>,
) -> Result<Judged> {
    let mut sure = Kept {
        change: Vec::new(),
        by_path: BTreeMap::new(),
    };
    let mut unsure = Kept {
        change: Vec::new(),
        by_path: BTreeMap::new(),
    };
    let mut left_out = Vec::new();
    for found in read_findings(text)? {
        let Some(entry) = resolution
            .entry(&found.rule_id)
            .filter(|entry| entry.rule.is_judgment())
        else {
            left_out.push(found.rule_id);
            continue;
        };
        let act_at = entry
            .rule
            .implementations
            .iter()
            .find_map(|implementation| implementation.act_at);
        let found = found.under(act_at);
        let kept = if found.unsure { &mut unsure } else { &mut sure };
        let finding = diagnostic(
            found,
            &entry.rule.summary,
            &entry.rule.id,
            Severity::from(entry.severity),
        );
        if finding.path.is_empty() {
            if entry.active {
                kept.change.push(finding);
            }
        } else {
            kept.by_path
                .entry(finding.path.clone())
                .or_default()
                .push(finding);
        }
    }
    let mut diagnostics = sure.change;
    diagnostics.extend(project::keep_judged(
        resolution,
        prefix,
        sure.by_path,
        &read,
    ));
    let mut lowered = unsure.change;
    lowered.extend(project::keep_judged(
        resolution,
        prefix,
        unsure.by_path,
        &read,
    ));
    for finding in &mut lowered {
        finding.severity = finding.severity.min(Severity::Info);
    }
    diagnostics.extend(lowered);
    Ok(Judged {
        diagnostics,
        left_out,
    })
}

#[cfg(test)]
mod tests;
