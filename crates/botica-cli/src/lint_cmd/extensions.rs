//! The extension rules of the catalogs of a project: one process for each rule, sent the files
//! where the rule applies.

use std::collections::{BTreeMap, HashMap};

use botica_catalog::{Implementation, Rule, Scope};
use botica_config::OnTimeout;
use botica_core::report::{ErrorKind, RunError};
use botica_core::walk::SourceFile;
use botica_core::Diagnostic;
use botica_ext::TimedOut;

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
    let mut read = Read::new();
    for (job, Done { rule, outcome }) in jobs.iter().zip(done) {
        if args.timings {
            timings::print_extension(&rule.id, &outcome.time);
        }
        let failed = outcome.failure.is_some();
        for (path, _) in &job.files {
            let finished = read.entry((rule.id.as_str(), path)).or_insert(true);
            *finished &= !failed;
        }
        found.extend(own_findings(rule, outcome.found));
        let warn = setup.pool.launcher().on_timeout == OnTimeout::Warn;
        let (late, failure): (Option<anyhow::Error>, _) = match outcome.failure {
            Some(error) if warn && error.downcast_ref::<TimedOut>().is_some() => {
                (Some(error), None)
            }
            failure => (None, failure),
        };
        if let Some(error) = late {
            eprintln!(
                "botica: warning: `{}` left out of the run: {error:#}",
                rule.id
            );
        }
        errors.extend(failure.map(|error| RunError {
            kind: ErrorKind::Extension,
            extension: Some(rule.id.clone()),
            path: None,
            message: format!("{error:#}"),
        }));
    }
    Ran {
        found: silenced(setup, sources, found, &read),
        errors,
    }
}

/// The files each extension rule was sent, as `(rule, path)`, and whether every process of the
/// rule that read the file finished.
type Read<'a> = HashMap<(&'a str, &'a str), bool>;

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
    let how = if job.rule.scope == Some(Scope::Project) {
        extension::Send::Project
    } else {
        extension::Send::Files { batch }
    };
    let outcome = match setup.pool.command_of(job.rule, job.implementation) {
        Ok(argv) => extension::run_rule(
            extension::Process {
                argv: &argv,
                dir: setup.pool.project_root(),
                selection: setup.pool.selection(job.rule),
                allowed: &setup.pool.sharing(&argv),
                deadlines: setup.pool.launcher().deadlines,
            },
            &job.files,
            how,
        ),
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

/// What the comments of the files leave of the findings, and the findings about the comments
/// that name an extension rule and silence nothing. The engine did not see these findings, so the
/// comments are read here.
fn silenced(
    setup: &Setup,
    sources: &[SourceFile],
    found: Vec<Diagnostic>,
    read: &Read<'_>,
) -> Vec<Diagnostic> {
    let policy = project::suppression_policy(&setup.resolution);
    let mut by_path: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for finding in found {
        by_path
            .entry(finding.path.clone())
            .or_default()
            .push(finding);
    }
    let mut kept = Vec::new();
    for source in sources {
        let path = source.path.as_str();
        let mut findings = by_path.remove(path).unwrap_or_default();
        if let Some(text) = source.text() {
            let ran = |slug: &str| match read.get(&(slug, path)) {
                Some(&finished) => finished.then_some(true),
                None => Some(false),
            };
            policy.report_extensions(path, text, &ran, &mut findings);
        }
        kept.extend(findings);
    }
    kept.extend(by_path.into_values().flatten());
    kept
}
