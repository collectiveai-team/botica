//! `botica lint --record` (ADR 0019): what a run appends to the history of the project, and the
//! rules that it evaluated.

use super::{git, LintArgs, Setup};
use source::{branch, head, is_object_name, origin};

mod source;
use crate::output::eprintln;
use botica_catalog::{Kind, Scope};
use botica_config::{HistoryConfig, Resolution};
use botica_core::fingerprint::{FingerprintKey, KEY_VARIABLE};
use botica_core::history::{self, FindingLine, RuleEntry, RunLine};
use botica_core::{language_of, Diagnostic};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

/// Whether the run records: `--record` or `[history] record`, unless `--no-record`.
pub(super) fn asked(args: &LintArgs, config: &HistoryConfig) -> bool {
    !args.no_record && (args.record || config.record)
}

/// What the run read (ADR 0019 decision 6).
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Full,
    Paths,
    Diff,
    Staged,
    Message,
    Judged,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Paths => "paths",
            Self::Diff => "diff",
            Self::Staged => "staged",
            Self::Message => "message",
            Self::Judged => "judged",
        }
    }

    /// The mode of a run that linted files: the last row of the table that its flags match. A run
    /// with no path from a directory below the root lints that directory alone, so it is `paths`.
    pub(super) fn of_files(args: &LintArgs, project_root: &Path, prefix: Option<&str>) -> Self {
        if args.staged {
            Self::Staged
        } else if args.diff.is_some() || args.diff_file.is_some() {
            Self::Diff
        } else if (args.paths.is_empty() && prefix.is_none())
            || is_the_root(&args.paths, project_root)
        {
            Self::Full
        } else {
            Self::Paths
        }
    }
}

fn is_the_root(paths: &[std::path::PathBuf], project_root: &Path) -> bool {
    let [path] = paths else {
        return false;
    };
    match (
        std::fs::canonicalize(path),
        std::fs::canonicalize(project_root),
    ) {
        (Ok(path), Ok(root)) => path == root,
        _ => false,
    }
}

/// The rules of a run that the plan gave at least one file, marked as the engine asks for the
/// plan of each file, with no lock.
pub(super) struct Evaluated {
    resolution: std::sync::Arc<Resolution>,
    prefix: Option<String>,
    with_astgrep: bool,
    index: HashMap<String, usize>,
    marks: Vec<AtomicBool>,
    /// While set, a plan that the engine asks for is not a file that the run lints.
    paused: AtomicBool,
}

impl Evaluated {
    pub(super) fn new(
        resolution: std::sync::Arc<Resolution>,
        prefix: Option<String>,
        with_astgrep: bool,
    ) -> Self {
        let index = resolution
            .entries()
            .iter()
            .enumerate()
            .map(|(position, entry)| (entry.rule.id.clone(), position))
            .collect();
        let marks = resolution
            .entries()
            .iter()
            .map(|_| AtomicBool::new(false))
            .collect();
        Self {
            resolution,
            prefix,
            with_astgrep,
            index,
            marks,
            paused: AtomicBool::new(false),
        }
    }

    /// Runs `task` without marking: what it asks of the planner lints no file.
    pub(super) fn paused<T>(&self, task: impl FnOnce() -> T) -> T {
        self.paused.store(true, Ordering::Relaxed);
        let done = task();
        self.paused.store(false, Ordering::Relaxed);
        done
    }

    /// Marks the native and ast-grep rules of scope `file` that run on the file at `path`,
    /// relative to the working directory, with the same selection as `project::planner`: the rules
    /// that the configuration applies at the path that the planner builds, `<prefix>/<path>` as
    /// written. An extension rule counts once its processes finish.
    pub(super) fn mark(&self, path: &str) {
        if self.paused.load(Ordering::Relaxed) {
            return;
        }
        let at_root = match self.prefix.as_deref() {
            Some(prefix) => format!("{prefix}/{path}"),
            None => path.to_owned(),
        };
        for file_rule in self.resolution.rules_for(&at_root, language_of(path)) {
            if matches!(file_rule.rule.scope, Some(Scope::Project | Scope::Commit)) {
                continue;
            }
            let runs = file_rule
                .implementations
                .iter()
                .any(|implementation| match implementation.kind {
                    // `registry::check_catalog` refuses a native rule that neither the registry
                    // nor the engine runs, so every native rule that applies here runs.
                    Kind::Native => true,
                    Kind::AstGrep => self.with_astgrep,
                    Kind::Extension | Kind::Judgment | Kind::External => false,
                });
            if let Some(&position) = self.index.get(file_rule.rule.id.as_str()) {
                if runs {
                    self.marks[position].store(true, Ordering::Relaxed);
                }
            }
        }
    }

    /// The slugs of the rules evaluated: those marked, and the active native rules of scope
    /// `project`, which run once in every run that lints files.
    pub(super) fn slugs(&self) -> Vec<String> {
        self.resolution
            .entries()
            .iter()
            .zip(&self.marks)
            .filter(|(entry, mark)| {
                mark.load(Ordering::Relaxed)
                    || (entry.active
                        && entry.rule.scope == Some(Scope::Project)
                        && !entry.rule.has_extension())
            })
            .map(|(entry, _)| entry.rule.id.clone())
            .collect()
    }
}

/// What a run gives the history once its findings are final.
pub(super) struct Run<'a> {
    pub started: SystemTime,
    pub mode: Mode,
    pub project_root: &'a Path,
    pub inside: super::inside::Inside<'a>,
    pub resolution: &'a Resolution,
    pub rules_fingerprint: String,
    pub history: &'a HistoryConfig,
    /// The rules that the run evaluated; a rule with a finding is added to them.
    pub evaluated: Vec<String>,
    /// The findings that the baseline hid, by rule.
    pub baselined: BTreeMap<String, usize>,
    /// The rules of an extension that failed: absent from `rules` even with findings.
    pub failed: Vec<String>,
    pub diagnostics: &'a [Diagnostic],
}

impl<'a> Run<'a> {
    /// What the run of `setup` gives the history, once its findings are final.
    pub(super) fn of(
        setup: &'a Setup,
        started: SystemTime,
        mode: Mode,
        evaluated: Vec<String>,
        diagnostics: &'a [Diagnostic],
    ) -> Self {
        Self {
            started,
            mode,
            project_root: &setup.project_root,
            inside: setup.inside(),
            resolution: &setup.resolution,
            rules_fingerprint: setup.rules_fingerprint.clone(),
            history: &setup.history,
            evaluated,
            baselined: BTreeMap::new(),
            failed: Vec::new(),
            diagnostics,
        }
    }

    /// Adds the extension rules whose processes all finished, and the rules of the findings of a
    /// `--ext` command that failed.
    pub(super) fn add_extensions(&mut self, finished: Vec<String>, failed: Vec<String>) {
        self.evaluated.extend(finished);
        self.failed.extend(failed);
    }

    /// Counts the findings that the baseline hid, by rule.
    pub(super) fn add_baselined(&mut self, hidden: &[Diagnostic]) {
        for diagnostic in hidden {
            *self.baselined.entry(diagnostic.rule.clone()).or_default() += 1;
        }
    }
}

/// What a run that linted files learned besides its findings.
pub(super) struct Silenced<'a> {
    /// The extension rules whose processes all finished.
    pub extensions_finished: Vec<String>,
    /// The rules of the findings of a `--ext` command that failed.
    pub failed_rules: Vec<String>,
    /// The findings that the baseline hid.
    pub baselined: &'a [Diagnostic],
}

/// Records a run that linted files, when the run records.
pub(super) fn files(
    args: &LintArgs,
    setup: &Setup,
    started: SystemTime,
    diagnostics: &[Diagnostic],
    silenced: Silenced<'_>,
) {
    let Some(evaluated) = &setup.evaluated else {
        return;
    };
    let mode = Mode::of_files(args, &setup.project_root, setup.prefix.as_deref());
    let mut run = Run::of(setup, started, mode, evaluated.slugs(), diagnostics);
    run.add_extensions(silenced.extensions_finished, silenced.failed_rules);
    run.add_baselined(silenced.baselined);
    write(&run);
}

/// Appends the run to the history. A history that cannot be written is a warning: the findings
/// and the exit code stay as they are.
pub(super) fn write(run: &Run<'_>) {
    if let Err(error) = append(run) {
        eprintln!("botica: warning: the run was not recorded in the history: {error:#}");
    }
}

fn append(run: &Run<'_>) -> anyhow::Result<()> {
    let value = std::env::var(KEY_VARIABLE)
        .ok()
        .filter(|value| !value.is_empty());
    if value.is_none() && run.history.require_key {
        anyhow::bail!("`[history] require-key` is set and `{KEY_VARIABLE}` is not");
    }
    let key = FingerprintKey::new(value.as_deref())?;
    let run_id = history::run_id(run.started);
    let mut findings: Vec<FindingLine> = run
        .diagnostics
        .iter()
        .map(|diagnostic| finding_line(run, &key, &run_id, diagnostic))
        .collect();
    history::number_occurrences(&mut findings);
    let commit = git::answer(&["rev-parse", "HEAD"]).filter(|sha| is_object_name(sha));
    let line = RunLine {
        v: history::VERSION,
        run_id,
        started_at: history::unix_seconds(run.started),
        head: head().or_else(|| commit.clone()),
        branch: branch(),
        commit,
        botica: env!("CARGO_PKG_VERSION").to_owned(),
        rules_fingerprint: run.rules_fingerprint.clone(),
        key_id: value.is_some().then(|| key.id()),
        mode: run.mode.as_str().to_owned(),
        origin: origin().to_owned(),
        rules: rules(run),
    };
    history::append(run.project_root, &line, &findings)
}

fn finding_line(
    run: &Run<'_>,
    key: &FingerprintKey,
    run_id: &str,
    diagnostic: &Diagnostic,
) -> FindingLine {
    // A path outside the root (`../x.py` from the root, `/home/…`, a link out of it) is dropped
    // (decision 7), and `fingerprints::stamp` hashed the message of its finding (decision 5).
    let path = match run.mode {
        Mode::Message => None,
        _ => run.inside.path(&diagnostic.path),
    };
    let fingerprint = diagnostic
        .fingerprint
        .clone()
        .unwrap_or_else(|| key.of_message(&diagnostic.rule, "", &diagnostic.message));
    FindingLine {
        v: history::VERSION,
        run_id: run_id.to_owned(),
        code: run
            .resolution
            .entry(&diagnostic.rule)
            .map(|entry| entry.rule.code.clone()),
        slug: diagnostic.rule.clone(),
        severity: diagnostic.severity.as_str().to_owned(),
        line: if path.is_some() { diagnostic.line } else { 0 },
        path: path.unwrap_or_default(),
        fingerprint,
        occurrence: 0,
    }
}

/// The entry of each rule evaluated, with the findings that the run reports and those that the
/// baseline hid. A rule with findings counts as evaluated, except an extension rule whose
/// processes did not all finish (decision 3). In a `judged` run these are the rules of the judge
/// that found something.
fn rules(run: &Run<'_>) -> BTreeMap<String, RuleEntry> {
    let mut rules: BTreeMap<String, RuleEntry> = run
        .evaluated
        .iter()
        .map(|slug| (slug.clone(), RuleEntry::default()))
        .collect();
    let counts = |slug: &str| {
        matches!(run.mode, Mode::Judged)
            || (!run.failed.iter().any(|failed| failed == slug)
                && run
                    .resolution
                    .entry(slug)
                    .is_none_or(|entry| !entry.rule.has_extension()))
    };
    for diagnostic in run.diagnostics {
        if let Some(entry) = rules.get_mut(&diagnostic.rule) {
            entry.hits += 1;
        } else if counts(&diagnostic.rule) {
            rules.entry(diagnostic.rule.clone()).or_default().hits += 1;
        }
    }
    for (slug, count) in &run.baselined {
        if let Some(entry) = rules.get_mut(slug) {
            entry.baselined += count;
        } else if counts(slug) {
            rules.entry(slug.clone()).or_default().baselined += count;
        }
    }
    rules
}
