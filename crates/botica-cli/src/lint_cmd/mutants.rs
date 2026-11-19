//! The mutants that survive a run of cargo-mutants (`botica lint --mutants <file>`): botica reads
//! them from its `outcomes.json` and reports each as a finding of `mutant-survived`, filtered by
//! the configuration and the suppression comments of the project.
//!
//! The format is the one of cargo-mutants 27.1.0 (`src/outcome.rs`, `src/mutant.rs`): a list
//! `outcomes`, each with a `scenario` that is `"Baseline"` or `{"Mutant": {...}}` and a
//! `summary`, `"MissedMutant"` for a mutant that the tests did not catch.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use botica_config::Resolution;
use botica_core::registry::MUTANT_SURVIVED;
use botica_core::report::Format;
use botica_core::{Diagnostic, Findings, Severity};
use serde::Deserialize;

use super::{exit_code, fingerprints, record, write_report, Diff, LintArgs, Setup};
use crate::project;

#[derive(Deserialize)]
struct Outcomes {
    outcomes: Vec<Outcome>,
}

#[derive(Deserialize)]
struct Outcome {
    scenario: Scenario,
    summary: String,
}

#[derive(Deserialize)]
enum Scenario {
    Baseline,
    Mutant(Mutant),
}

#[derive(Deserialize)]
struct Mutant {
    name: String,
    file: String,
    span: Span,
}

#[derive(Deserialize)]
struct Span {
    start: LineColumn,
    end: LineColumn,
}

#[derive(Deserialize)]
struct LineColumn {
    line: usize,
    column: usize,
}

const MISSED: &str = "MissedMutant";
const SUCCESS: &str = "Success";

/// The mutants of the report that the tests did not catch. A report whose baseline failed tested
/// no mutant, and is refused rather than read as a clean run. A mutant whose file is not a
/// relative path inside the tree, such as `/etc/x.rs`, `C:/x.rs` or `../x.rs`, is left out.
fn survivors(text: &str) -> Result<Vec<Mutant>> {
    let report: Outcomes = serde_json::from_str(text)
        .context("the mutants are not a report of cargo-mutants (`mutants.out/outcomes.json`)")?;
    let mut survivors = Vec::new();
    for outcome in report.outcomes {
        match outcome.scenario {
            Scenario::Baseline if outcome.summary != SUCCESS => bail!(
                "the baseline of cargo-mutants did not pass (`{}`): the report tested no mutant",
                outcome.summary
            ),
            Scenario::Baseline => {}
            Scenario::Mutant(mutant) if outcome.summary == MISSED && inside(&mutant.file) => {
                survivors.push(mutant);
            }
            Scenario::Mutant(_) => {}
        }
    }
    Ok(survivors)
}

/// Whether `path` is relative, names no drive or URI scheme, holds no control character and stays
/// inside the tree it is relative to.
fn inside(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && !path.split('/').next().unwrap_or_default().contains(':')
        && path.split('/').all(|part| part != "..")
}

/// What the name of a mutant says after its location, `replace == with != in is_even`, on one
/// line: a control character of the report becomes a space.
fn change_of(mutant: &Mutant) -> String {
    let location = format!(
        "{}:{}:{}: ",
        mutant.file, mutant.span.start.line, mutant.span.start.column
    );
    mutant
        .name
        .strip_prefix(location.as_str())
        .unwrap_or(&mutant.name)
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

fn diagnostic(mutant: &Mutant, severity: Severity) -> Diagnostic {
    let (line, column) = (mutant.span.start.line, mutant.span.start.column);
    // The end of a span of cargo-mutants is exclusive; that of a finding is the last character.
    let (end_line, end_column) = (
        mutant.span.end.line.max(line),
        mutant.span.end.column.saturating_sub(1).max(1),
    );
    Diagnostic {
        path: mutant.file.clone(),
        rule: MUTANT_SURVIVED.to_owned(),
        severity,
        start: 0,
        end: 0,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        fingerprint: None,
        measure: None,
        message: format!(
            "The tests pass with this mutant of cargo-mutants: {}. Add a test that fails on it, \
             or delete the code it changes",
            change_of(mutant)
        ),
    }
}

/// Reads the report of cargo-mutants and keeps what the project reports of its surviving mutants:
/// none when `mutant-survived` is not active, else the findings at the paths and in the languages
/// the configuration applies it to, with its severity there, minus what a comment on their line or
/// an `ignore-file` comment silences (the walk reports one with no reason). `read` gives the text
/// of a file.
pub fn filter(
    resolution: &Arc<Resolution>,
    prefix: Option<&str>,
    text: &str,
    read: impl Fn(&str) -> Option<String>,
) -> Result<Findings> {
    let survivors = survivors(text)?;
    let Some(entry) = resolution
        .entry(MUTANT_SURVIVED)
        .filter(|entry| entry.active)
    else {
        return Ok(Findings::default());
    };
    let severity = Severity::from(entry.severity);
    let mut by_path: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for mutant in &survivors {
        by_path
            .entry(mutant.file.clone())
            .or_default()
            .push(diagnostic(mutant, severity));
    }
    Ok(project::keep_judged(resolution, prefix, by_path, read))
}

/// `botica lint --mutants <outcomes>`: reports the surviving mutants that the project keeps, on
/// the lines that `diff` adds when there is one, and records them when the run records.
pub(super) fn run(
    args: &LintArgs,
    format: Format,
    setup: &Setup,
    outcomes: &Path,
    diff: Option<&Diff>,
    started: SystemTime,
) -> Result<ExitCode> {
    let text = std::fs::read_to_string(outcomes)
        .with_context(|| format!("cannot read `{}`", outcomes.display()))?;
    let mut survived = filter(&setup.resolution, setup.prefix.as_deref(), &text, |path| {
        std::fs::read_to_string(path).ok()
    })?;
    let keeps = |diagnostic: &Diagnostic| diff.is_none_or(|diff| diff.changes.keeps(diagnostic));
    survived.found.retain(&keeps);
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
        &mut survived.found,
    );
    if setup.evaluated.is_some() {
        record_run(setup, started, &survived, keeps);
    }
    write_report(args, format, setup, &survived.found, &[])?;
    Ok(exit_code(&[], &survived.found, args))
}

/// Records a `mutants` run: the surviving mutants that the run reports, and those that a comment
/// silenced, which `keeps` filters as it filters the reported ones. `mutant-survived` is the one
/// rule evaluated when it is active.
fn record_run(
    setup: &Setup,
    started: SystemTime,
    survived: &Findings,
    keeps: impl Fn(&Diagnostic) -> bool,
) {
    let mut hidden = record::Hidden::new(survived.silenced.clone());
    hidden.keep_only(keeps);
    let evaluated = setup
        .resolution
        .entry(MUTANT_SURVIVED)
        .filter(|entry| entry.active)
        .map(|entry| entry.rule.id.clone())
        .into_iter()
        .collect();
    let mut run = record::Run::of(
        setup,
        started,
        record::Mode::Mutants,
        evaluated,
        &survived.found,
    );
    run.add_hidden(hidden, setup.prefix.as_deref());
    record::write(&run);
}
