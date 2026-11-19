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

use super::{exit_code, fingerprints, record, write_report, Diff, LintArgs, Setup};
use crate::output::eprintln;
use crate::project;
use botica_core::report::Format;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::SystemTime;

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
    /// The findings that an `ignore-file` comment silenced.
    pub silenced: Vec<Diagnostic>,
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

/// The findings of several runs of a judge on one change, as one list. A finding is sure when a
/// majority of the runs give it as sure, after `act-at`; otherwise it is unsure. Its note lists
/// the probability of each run that gave it, so a reader sees how far the runs were apart.
fn combine(runs: Vec<Vec<Found>>, act_at: impl Fn(&str) -> Option<Probability>) -> Vec<Found> {
    let count = runs.len();
    if count == 1 {
        return runs
            .into_iter()
            .flatten()
            .map(|found| {
                let act_at = act_at(&found.rule_id);
                found.under(act_at)
            })
            .collect();
    }
    let mut merged: BTreeMap<(String, String), (Found, Vec<String>, usize)> = BTreeMap::new();
    for found in runs.into_iter().flatten() {
        let act_at = act_at(&found.rule_id);
        let found = found.under(act_at);
        let shown = found
            .probability
            .map_or_else(|| "?".to_owned(), |p| format!("{p:.2}"));
        let shown = format!("{shown}{}", if found.unsure { "" } else { " act" });
        let sure = usize::from(!found.unsure);
        match merged.entry((found.rule_id.clone(), found.path.clone())) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let (_, probabilities, sure_runs) = entry.get_mut();
                probabilities.push(shown);
                *sure_runs += sure;
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((found, vec![shown], sure));
            }
        }
    }
    merged
        .into_values()
        .map(|(mut found, probabilities, sure_runs)| {
            found.unsure = sure_runs * 2 <= count;
            found.note = Some(format!(
                "{sure_runs} of {count} runs of abide see the change break it; probabilities {}",
                probabilities.join(", ")
            ));
            found
        })
        .collect()
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
        fingerprint: None,
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
/// `texts` holds one report per run of the judge on the same change; with several, a finding
/// keeps its severity only when a majority of the runs give it as sure (`combine`). `read` gives
/// the text of a file.
pub fn filter(
    resolution: &Arc<Resolution>,
    prefix: Option<&str>,
    texts: &[String],
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
    let runs = texts
        .iter()
        .map(|text| read_findings(text))
        .collect::<Result<Vec<_>>>()?;
    let act_at = |rule: &str| {
        resolution
            .entry(rule)
            .filter(|entry| entry.rule.is_judgment())
            .and_then(|entry| {
                entry
                    .rule
                    .implementations
                    .iter()
                    .find_map(|implementation| implementation.act_at)
            })
    };
    for found in combine(runs, act_at) {
        let Some(entry) = resolution
            .entry(&found.rule_id)
            .filter(|entry| entry.rule.is_judgment())
        else {
            left_out.push(found.rule_id);
            continue;
        };
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
    let sure = project::keep_judged(resolution, prefix, sure.by_path, &read);
    diagnostics.extend(sure.found);
    let mut silenced = sure.silenced;
    let mut lowered = unsure.change;
    let unsure = project::keep_judged(resolution, prefix, unsure.by_path, &read);
    lowered.extend(unsure.found);
    silenced.extend(unsure.silenced);
    for finding in &mut lowered {
        finding.severity = finding.severity.min(Severity::Info);
    }
    diagnostics.extend(lowered);
    Ok(Judged {
        diagnostics,
        silenced,
        left_out,
    })
}

/// Reports the findings of the reports of `--judged`, as `botica lint` reports its own.
pub(super) fn run(
    args: &LintArgs,
    format: Format,
    setup: &Setup,
    reports: &[PathBuf],
    diff: Option<&Diff>,
    started: SystemTime,
) -> Result<ExitCode> {
    let texts = reports
        .iter()
        .map(|report| {
            std::fs::read_to_string(report)
                .with_context(|| format!("cannot read `{}`", report.display()))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut judged = filter(&setup.resolution, setup.prefix.as_deref(), &texts, |path| {
        std::fs::read_to_string(path).ok()
    })?;
    let keeps = |diagnostic: &Diagnostic| diff.is_none_or(|diff| diff.changes.keeps(diagnostic));
    judged.diagnostics.retain(&keeps);
    let root = std::env::current_dir()?;
    fingerprints::stamp(
        format,
        setup.evaluated.is_some(),
        &setup.inside(),
        fingerprints::Read::Files {
            sources: &[],
            root: &root,
            staged: diff.is_some_and(Diff::is_staged),
        },
        &mut judged.diagnostics,
    );
    for rule in &judged.left_out {
        eprintln!(
            "botica: `{rule}` is not a judgment rule of the catalog: its finding is left out"
        );
    }
    if setup.evaluated.is_some() {
        record::judged(setup, started, &judged, keeps);
    }
    write_report(args, format, setup, &judged.diagnostics, &[])?;
    Ok(exit_code(&[], &judged.diagnostics, args))
}

#[cfg(test)]
mod tests;
