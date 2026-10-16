//! The extension rules of the catalogs of a project: one process for each rule, sent the files
//! where the rule applies.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use botica_catalog::{Implementation, Rule, Runtime};
use botica_core::report::{ErrorKind, RunError};
use botica_core::walk::SourceFile;
use botica_core::Diagnostic;

use super::{LintArgs, Setup};
use crate::project::{self, extensions_at};
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
        for (rule, implementation) in
            extensions_at(&setup.resolution, setup.prefix.as_deref(), &source.path)
        {
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
    let outcome = match command_of(&setup.catalog_dirs, job.rule, job.implementation) {
        Ok(argv) => extension::run_files(&argv, &setup.project_root, &job.files, batch),
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

/// What runs the implementation: `uv run --script` on the script of a Python rule, `node` on the
/// script of a Node rule, the command of a rule that names one.
fn command_of(
    catalog_dirs: &BTreeMap<String, PathBuf>,
    rule: &Rule,
    implementation: &Implementation,
) -> Result<Vec<String>, String> {
    let named = |what: &str| format!("`{}` has no {what}", rule.id);
    match implementation.runtime {
        Some(Runtime::Command) => implementation
            .command
            .clone()
            .ok_or_else(|| named("`command`")),
        Some(runtime) => {
            let script =
                script_of(catalog_dirs, rule, implementation).ok_or_else(|| named("script"))?;
            let script = script.to_string_lossy().into_owned();
            Ok(match runtime {
                Runtime::Python => ["uv", "run", "--script"]
                    .map(str::to_owned)
                    .into_iter()
                    .chain([script])
                    .collect(),
                _ => vec!["node".to_owned(), script],
            })
        }
        None => Err(named("runtime")),
    }
}

/// The script of the rule, in its folder of `<catalog>/<family>/<slug>/`, where the catalog is.
fn script_of(
    catalog_dirs: &BTreeMap<String, PathBuf>,
    rule: &Rule,
    implementation: &Implementation,
) -> Option<PathBuf> {
    let catalog = rule.catalog.as_deref()?;
    let slug = rule.id.strip_prefix(catalog)?.strip_prefix('/')?;
    let folder = catalog_dirs
        .get(catalog)?
        .join(rule.family.as_deref()?)
        .join(slug);
    Some(folder.join(implementation.file.as_deref()?))
}

/// The findings that are about the rule. A finding about another rule is left out, and said once.
fn own_findings(rule: &Rule, found: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let (own, foreign): (Vec<Diagnostic>, Vec<Diagnostic>) = found
        .into_iter()
        .partition(|finding| finding.rule == rule.id);
    let mut named: Vec<&str> = foreign
        .iter()
        .map(|finding| finding.rule.as_str())
        .collect();
    named.sort_unstable();
    named.dedup();
    for other in named {
        eprintln!(
            "botica: `{}` answered with a finding of `{other}`: left out",
            rule.id
        );
    }
    own
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
