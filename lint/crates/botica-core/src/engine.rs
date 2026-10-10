use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};
use crate::suppress::Policy;
use crate::timings::{RuleTime, TimedKind, Timings};
use anyhow::Result;
use botica_astgrep::AstGrepSet;
use botica_ext::{ExtensionHost, FileIn};
use std::collections::HashMap;

/// For a path relative to the project root: the indexes of the native rules that apply to it.
pub type NativePlanner = Box<dyn Fn(&str) -> Vec<usize> + Send + Sync>;

/// What runs on one file: the indexes of its native rules, and whether the ast-grep set may match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub native: Vec<usize>,
    pub astgrep: bool,
    /// Whether anything looks inside the file. When it is `false` the rules go by the path alone,
    /// and the file need not be opened: that is how a binary file or a file that is not UTF-8 gets
    /// its findings about its name.
    pub content: bool,
}

/// Decides, for an entry the walk met (its path and whether it is a directory), if it is left out.
pub type EntryExclusion = Box<dyn Fn(&str, bool) -> bool + Send + Sync>;

/// Decides, for a path given by name, if it is left out.
pub type PathExclusion = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Which paths the walk leaves out (`extend-exclude`). Paths are relative to where botica runs.
pub struct Exclusion {
    /// A directory that matches is skipped with all that is inside.
    pub entry: EntryExclusion,
    /// The path or a directory above it matches.
    pub path: PathExclusion,
}

pub struct Engine {
    native: Vec<Box<dyn NativeRule>>,
    astgrep: AstGrepSet,
    planner: Option<NativePlanner>,
    suppressions: Option<Policy>,
    exclusion: Option<Exclusion>,
    timings: Option<Timings>,
}

impl Engine {
    /// Without a planner every native rule applies to every path.
    pub fn new(native: Vec<Box<dyn NativeRule>>, astgrep: AstGrepSet) -> Self {
        Self {
            native,
            astgrep,
            planner: None,
            suppressions: None,
            exclusion: None,
            timings: None,
        }
    }

    /// Starts timing each rule (`rule_times`). Slows the run down a little: leave it off unless
    /// asked.
    pub fn with_timings(mut self) -> Self {
        self.timings = Some(Timings::new(self.native.len()));
        self.astgrep.time_rules();
        self
    }

    /// What each rule cost so far; empty unless `with_timings` was called. In no useful order.
    pub fn rule_times(&self) -> Vec<RuleTime> {
        let Some(timings) = &self.timings else {
            return Vec::new();
        };
        let mut times: Vec<RuleTime> = self
            .native
            .iter()
            .enumerate()
            .map(|(slot, rule)| {
                let (nanos, files) = timings.total(slot);
                RuleTime {
                    rule: rule.id().to_owned(),
                    kind: TimedKind::Native,
                    nanos,
                    files,
                }
            })
            .collect();
        times.extend(self.astgrep.rule_times().into_iter().map(|time| RuleTime {
            rule: time.id,
            kind: TimedKind::AstGrep,
            nanos: time.nanos,
            files: time.files,
        }));
        let (nanos, files) = timings.total(timings.suppressions_slot());
        times.push(RuleTime {
            rule: "suppression-comments".to_owned(),
            kind: TimedKind::Suppressions,
            nanos,
            files,
        });
        times
    }

    /// Without an exclusion the walk leaves nothing out.
    pub fn with_exclusion(mut self, exclusion: Exclusion) -> Self {
        self.exclusion = Some(exclusion);
        self
    }

    /// Whether the walk should skip this entry (and, for a directory, all that is inside).
    pub fn skips_entry(&self, path: &str, is_dir: bool) -> bool {
        self.exclusion
            .as_ref()
            .is_some_and(|exclusion| (exclusion.entry)(path, is_dir))
    }

    /// Whether a path given by name is excluded.
    pub fn skips_path(&self, path: &str) -> bool {
        self.exclusion
            .as_ref()
            .is_some_and(|exclusion| (exclusion.path)(path))
    }

    pub fn has_exclusion(&self) -> bool {
        self.exclusion.is_some()
    }

    /// Without a policy, comments silence nothing.
    pub fn with_suppressions(mut self, policy: Policy) -> Self {
        self.suppressions = Some(policy);
        self
    }

    pub fn with_planner(mut self, planner: NativePlanner) -> Self {
        self.planner = Some(planner);
        self
    }

    /// `None` when nothing applies to `path`, so the file need not even be read.
    ///
    /// With no rule of code to run at all (the selection names only the rules about suppression
    /// comments), the files that can hold such comments are still read: the comments are the
    /// whole job.
    pub fn plan(&self, path: &str) -> Option<Plan> {
        let native = match &self.planner {
            Some(planner) => planner(path),
            None => (0..self.native.len()).collect(),
        };
        let astgrep = self.astgrep.covers(path.as_ref());
        let comments_only = self.native.is_empty()
            && self.astgrep.is_empty()
            && self
                .suppressions
                .as_ref()
                .is_some_and(|policy| policy.reads(path));
        let content = astgrep
            || native
                .iter()
                .any(|&index| self.native[index].needs() != Needs::Path)
            || self
                .suppressions
                .as_ref()
                .is_some_and(|policy| policy.can_silence(path));
        (!native.is_empty() || astgrep || comments_only).then_some(Plan {
            native,
            astgrep,
            content,
        })
    }

    pub fn astgrep(&self) -> &AstGrepSet {
        &self.astgrep
    }

    /// Lints text the caller already holds. Does no I/O.
    pub fn lint_source(&self, path: &str, text: &str) -> Vec<Diagnostic> {
        match self.plan(path) {
            Some(plan) => self.lint_planned(path, text, &plan),
            None => Vec::new(),
        }
    }

    /// Like `lint_source` for a caller that already asked for the plan (to decide whether to read).
    pub fn lint_planned(&self, path: &str, text: &str, plan: &Plan) -> Vec<Diagnostic> {
        self.lint_document(&Document::new(path, text), plan)
    }

    /// Lints a file as it was read, UTF-8 or not. Does no I/O.
    pub fn lint_bytes(&self, path: &str, bytes: &[u8]) -> Vec<Diagnostic> {
        match self.plan(path) {
            Some(plan) => self.lint_bytes_planned(path, bytes, &plan),
            None => Vec::new(),
        }
    }

    /// Like `lint_bytes` for a caller that already asked for the plan.
    ///
    /// A file that is not UTF-8 gets only the native rules that do not need its text: ast-grep
    /// cannot parse it and the suppression comments cannot be read.
    pub fn lint_bytes_planned(&self, path: &str, bytes: &[u8], plan: &Plan) -> Vec<Diagnostic> {
        self.lint_document(&Document::from_bytes(path, bytes), plan)
    }

    /// What the rules that go by the name alone find in a file that looks binary and is not read.
    pub fn lint_unread(&self, path: &str, plan: &Plan) -> Vec<Diagnostic> {
        let doc = Document::from_bytes(path, &[]);
        let mut out = Vec::new();
        for &index in &plan.native {
            if self.native[index].needs() == Needs::Path {
                self.native[index].check(&doc, &mut out);
            }
        }
        out.sort_by(|a, b| (a.start, &a.rule).cmp(&(b.start, &b.rule)));
        out
    }

    fn lint_document(&self, doc: &Document<'_>, plan: &Plan) -> Vec<Diagnostic> {
        let path = doc.path;
        let text = doc.text();
        let mut out = Vec::new();
        for &index in &plan.native {
            if text.is_none() && self.native[index].needs() == Needs::Text {
                continue;
            }
            match &self.timings {
                Some(timings) => timings.measure(index, || self.native[index].check(doc, &mut out)),
                None => self.native[index].check(doc, &mut out),
            }
        }
        let hits = match text {
            Some(text) if plan.astgrep => self.astgrep.check(path.as_ref(), text),
            _ => Vec::new(),
        };
        for hit in hits {
            let (line, column) = doc.position(hit.start);
            let (end_line, end_column) = doc.position(hit.end);
            out.push(Diagnostic {
                path: path.to_owned(),
                rule: hit.rule_id,
                severity: Severity::parse(hit.severity),
                start: hit.start,
                end: hit.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: hit.message,
            });
        }
        if let (Some(policy), Some(_)) = (&self.suppressions, text) {
            let is_active = |slug: &str| {
                plan.native
                    .iter()
                    .any(|&index| self.native[index].id() == slug)
                    || (plan.astgrep && policy.is_astgrep(slug))
            };
            match &self.timings {
                Some(timings) => timings.measure(timings.suppressions_slot(), || {
                    policy.apply(doc, &is_active, &mut out);
                }),
                None => policy.apply(doc, &is_active, &mut out),
            }
        }
        out.sort_by(|a, b| (a.start, &a.rule).cmp(&(b.start, &b.rule)));
        out
    }
}

pub struct ExtensionRun {
    host: ExtensionHost,
    languages: Vec<String>,
}

pub fn language_of(path: &str) -> Option<&'static str> {
    match path.rsplit_once('.')?.1 {
        "py" | "pyi" => Some("python"),
        "ts" | "tsx" | "mts" | "cts" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        _ => None,
    }
}

impl ExtensionRun {
    pub fn start(command: &[String]) -> Result<Self> {
        let host = ExtensionHost::spawn(command)?;
        let languages = host
            .rules
            .iter()
            .flat_map(|rule| rule.languages.iter().cloned())
            .collect();
        Ok(Self { host, languages })
    }

    pub fn lint<'a>(
        &mut self,
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
        batch_size: usize,
    ) -> Result<Vec<Diagnostic>> {
        let wanted: Vec<FileIn<'_>> = files
            .into_iter()
            .filter(|(path, _)| {
                language_of(path).is_some_and(|l| self.languages.iter().any(|x| x == l))
            })
            .map(|(path, text)| FileIn { path, text })
            .collect();
        let mut out = Vec::new();
        for batch in wanted.chunks(batch_size.max(1)) {
            let texts: HashMap<&str, &str> = batch.iter().map(|f| (f.path, f.text)).collect();
            for wire in self.host.lint_files(batch)? {
                let text = texts.get(wire.path.as_str()).copied().unwrap_or("");
                let doc = Document::new(&wire.path, text);
                let (line, column) = doc.position(wire.start);
                let (end_line, end_column) = doc.position(wire.end);
                out.push(Diagnostic {
                    path: wire.path,
                    rule: wire.rule,
                    severity: Severity::parse(&wire.severity),
                    start: wire.start,
                    end: wire.end,
                    line,
                    column,
                    end_line,
                    end_column,
                    file_level: false,
                    message: wire.message,
                });
            }
        }
        Ok(out)
    }

    pub fn finish(self) -> Result<()> {
        self.host.shutdown()
    }
}
