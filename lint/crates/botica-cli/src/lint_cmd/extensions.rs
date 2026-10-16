//! The extension rules of the catalogs of a project: one process for each rule, sent the files
//! where the rule applies.

use std::collections::{BTreeMap, HashMap};

use botica_catalog::{Implementation, Rule};
use botica_core::report::{ErrorKind, RunError};
use botica_core::walk::SourceFile;
use botica_core::Diagnostic;

use super::{LintArgs, Setup};
use crate::extension_pool::own_findings;
use crate::project;
use crate::{extension, timings};

/// What the extension rules found, and why some of them did not finish.
pub(super) struct Ran {
    pub found: Vec<Diagnostic>,
    pub errors: Vec<RunError>,
}

/// One process: a rule and its implementation, and the files it is sent, as `(path, text)`.
struct Job<'a> {
    rule: &'a Rule,
    implementation: &'a Implementation,
    files: Vec<(&'a str, &'a str)>,
}

/// What one process gave back.
struct Done<'a> {
    rule: &'a Rule,
    outcome: extension::Outcome,
}

pub(super) fn run(args: &LintArgs, setup: &Setup, sources: &[SourceFile]) -> Ran {
    let jobs = jobs(setup, sources);
    let done: Vec<Done<'_>> = std::thread::scope(|scope| {
        let started: Vec<_> = jobs
            .iter()
            .map(|job| scope.spawn(|| execute(job, setup, args.batch)))
            .collect();
        started
            .into_iter()
            .map(|handle| handle.join().expect("an extension thread does not panic"))
            .collect()
    });

    let mut found = Vec::new();
    let mut errors = Vec::new();
    for Done { rule, outcome } in done {
        if args.timings {
            timings::print_extension(&rule.id, &outcome.time);
        }
        found.extend(own_findings(rule, outcome.found));
        errors.extend(outcome.failure.map(|error| RunError {
            kind: ErrorKind::Extension,
            extension: Some(rule.id.clone()),
            path: None,
            message: format!("{error:#}"),
        }));
    }
    Ran {
        found: silenced(setup, sources, found),
        errors,
    }
}

/// The processes to start: for each active extension rule, the files where it applies. The
/// files are in the order of the sources, and the processes in the order of the rules.
fn jobs<'a>(setup: &'a Setup, sources: &'a [SourceFile]) -> Vec<Job<'a>> {
    let mut jobs: BTreeMap<(&str, usize), Job<'a>> = BTreeMap::new();
    for source in sources {
        let Some(text) = source.text() else {
            continue;
        };
        for (rule, implementation) in setup.pool.at(&source.path) {
            let position = rule
                .implementations
                .iter()
                .position(|candidate| std::ptr::eq(candidate, implementation))
                .unwrap_or_default();
            jobs.entry((rule.id.as_str(), position))
                .or_insert_with(|| Job {
                    rule,
                    implementation,
                    files: Vec::new(),
                })
                .files
                .push((source.path.as_str(), text));
        }
    }
    jobs.into_values().collect()
}

fn execute<'a>(job: &Job<'a>, setup: &Setup, batch: usize) -> Done<'a> {
    let outcome = match setup.pool.command_of(job.rule, job.implementation) {
        Ok(argv) => extension::run_files(&argv, setup.pool.project_root(), &job.files, batch),
        Err(message) => extension::Outcome {
            found: Vec::new(),
            failure: Some(anyhow::anyhow!(message)),
            time: extension::ExtensionTime::default(),
        },
    };
    Done {
        rule: job.rule,
        outcome,
    }
}

/// What the comments of the files leave of the findings. The engine did not see these findings,
/// so the comments are read here.
fn silenced(setup: &Setup, sources: &[SourceFile], found: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let texts: HashMap<&str, &str> = sources
        .iter()
        .filter_map(|source| Some((source.path.as_str(), source.text()?)))
        .collect();
    let policy = project::suppression_policy(&setup.resolution);
    let mut by_path: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for finding in found {
        by_path
            .entry(finding.path.clone())
            .or_default()
            .push(finding);
    }
    let mut kept = Vec::new();
    for (path, mut findings) in by_path {
        if let Some(text) = texts.get(path.as_str()) {
            policy.silence(&path, text, &mut findings);
        }
        kept.extend(findings);
    }
    kept
}
