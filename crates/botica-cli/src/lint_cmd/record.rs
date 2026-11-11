//! `botica lint --record` (ADR 0019): what a run appends to the history of the project, and the
//! rules that it evaluated.

use super::baseline::from_root;
use super::LintArgs;
use crate::output::eprintln;
use botica_catalog::{Kind, Scope};
use botica_config::{HistoryConfig, Resolution};
use botica_core::fingerprint::{FingerprintKey, KEY_VARIABLE};
use botica_core::history::{self, FindingLine, RuleEntry, RunLine};
use botica_core::{language_of, Diagnostic};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::process::{Command, Stdio};
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

    /// The mode of a run that linted files: the last row of the table that its flags match.
    pub(super) fn of_files(args: &LintArgs, project_root: &Path) -> Self {
        if args.staged {
            Self::Staged
        } else if args.diff.is_some() || args.diff_file.is_some() {
            Self::Diff
        } else if args.paths.is_empty() || is_the_root(&args.paths, project_root) {
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

/// Who started the run: CI before a hook, so that `prek run` inside a CI job counts as CI.
fn origin() -> &'static str {
    let is = |name: &str, value: &str| std::env::var(name).is_ok_and(|found| found == value);
    if is("GITHUB_ACTIONS", "true") || is("GITLAB_CI", "true") {
        "ci"
    } else if is("PRE_COMMIT", "1") {
        "hook"
    } else {
        "cli"
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
        }
    }

    /// Marks the native and ast-grep rules of scope `file` that run on the file at `path`,
    /// relative to the working directory. An extension rule counts once its processes finish.
    pub(super) fn mark(&self, path: &str) {
        let at_root = from_root(self.prefix.as_deref(), path);
        for file_rule in self.resolution.rules_for(&at_root, language_of(path)) {
            if matches!(file_rule.rule.scope, Some(Scope::Project | Scope::Commit)) {
                continue;
            }
            let runs = file_rule
                .implementations
                .iter()
                .any(|implementation| match implementation.kind {
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
    pub prefix: Option<&'a str>,
    pub resolution: &'a Resolution,
    pub rules_fingerprint: String,
    pub history: &'a HistoryConfig,
    /// The rules that the run evaluated; a rule with a finding is added to them.
    pub evaluated: Vec<String>,
    /// The findings that the baseline hid, by rule.
    pub baselined: BTreeMap<String, usize>,
    pub diagnostics: &'a [Diagnostic],
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
    let commit = git(run.project_root, &["rev-parse", "HEAD"]).filter(|sha| is_object_name(sha));
    let line = RunLine {
        v: history::VERSION,
        run_id,
        started_at: history::unix_seconds(run.started),
        head: head().or_else(|| commit.clone()),
        branch: branch(run.project_root),
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
    let whole = matches!(run.mode, Mode::Message) || diagnostic.path.is_empty();
    let fingerprint = match (&diagnostic.fingerprint, whole) {
        (Some(fingerprint), _) => fingerprint.clone(),
        (None, true) => key.of_message(&diagnostic.rule, "", &diagnostic.message),
        (None, false) => key.of_file(&diagnostic.rule, &from_root(run.prefix, &diagnostic.path)),
    };
    FindingLine {
        v: history::VERSION,
        run_id: run_id.to_owned(),
        code: run
            .resolution
            .entry(&diagnostic.rule)
            .map(|entry| entry.rule.code.clone()),
        slug: diagnostic.rule.clone(),
        severity: diagnostic.severity.as_str().to_owned(),
        path: if whole {
            String::new()
        } else {
            from_root(run.prefix, &diagnostic.path)
        },
        line: if whole { 0 } else { diagnostic.line },
        fingerprint,
        occurrence: 0,
    }
}

/// The entry of each rule evaluated, with the findings that the run reports and those that the
/// baseline hid.
fn rules(run: &Run<'_>) -> BTreeMap<String, RuleEntry> {
    let mut rules: BTreeMap<String, RuleEntry> = run
        .evaluated
        .iter()
        .map(|slug| (slug.clone(), RuleEntry::default()))
        .collect();
    for diagnostic in run.diagnostics {
        rules.entry(diagnostic.rule.clone()).or_default().hits += 1;
    }
    for (slug, count) in &run.baselined {
        rules.entry(slug.clone()).or_default().baselined += count;
    }
    rules
}

/// The commit under review in CI: the head of the pull request on GitHub, the head of the source
/// branch of a pipeline for merged results on GitLab.
fn head() -> Option<String> {
    let github = std::env::var("GITHUB_EVENT_NAME")
        .is_ok_and(|event| event.starts_with("pull_request"))
        .then(|| std::env::var("GITHUB_EVENT_PATH").ok())
        .flatten()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|event| {
            event["pull_request"]["head"]["sha"]
                .as_str()
                .map(str::to_owned)
        });
    github
        .or_else(|| set("CI_MERGE_REQUEST_SOURCE_BRANCH_SHA"))
        .filter(|sha| is_object_name(sha))
}

fn branch(project_root: &Path) -> Option<String> {
    set("GITHUB_HEAD_REF")
        .or_else(|| git(project_root, &["branch", "--show-current"]))
        .or_else(|| set("CI_MERGE_REQUEST_SOURCE_BRANCH_NAME"))
        .or_else(|| set("CI_COMMIT_REF_NAME"))
}

/// The value of the variable `name`; an empty one counts as unset.
fn set(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The object name of a SHA-1 or a SHA-256 repository.
fn is_object_name(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// What git prints for `args` at the root of the project, trimmed; `None` when it fails or prints
/// nothing.
fn git(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (output.status.success() && !text.is_empty()).then(|| text.to_owned())
}
