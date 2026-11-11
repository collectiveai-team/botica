//! The readings that pair a hook run with a CI run (ADR 0019 decision 8): "Caught before the
//! commit" and "Reached CI".

use super::ancestry::Ancestry;
use super::lines::Run;
use super::read::History;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Why a rule of two paired runs is left out of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// A run records `null` as the hash of the rule, so nothing says that the rule is the same.
    Unhashed,
    /// The two runs record different hashes: the version, the rule or its settings changed.
    Changed,
}

/// What the paired readings found.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Paired {
    /// The findings that a hook run held and the paired CI run did not, by rule.
    pub caught: BTreeMap<String, u64>,
    /// The findings of a CI run that no hook run before it held, by rule.
    pub reached: BTreeMap<String, u64>,
    /// The hook runs of mode `staged` in the window.
    pub hook_runs: usize,
    /// The CI runs of mode `diff` in the window.
    pub ci_runs: usize,
    /// The hook runs that have a CI run to pair with.
    pub paired_hooks: usize,
    /// The CI runs that have a hook run before them.
    pub paired_ci: usize,
    pub not_comparable: BTreeMap<String, BTreeSet<Reason>>,
}

/// The fingerprints of the findings of each rule in each run.
type Held<'a> = HashMap<(usize, &'a str), HashMap<&'a str, HashSet<&'a str>>>;

pub fn pair(history: &History, window: &[&Run], ancestry: &dyn Ancestry) -> Paired {
    let runs_of = |origin: &str, mode: &str| -> Vec<&Run> {
        window
            .iter()
            .copied()
            .filter(|run| run.is(origin, mode) && named(run))
            .collect()
    };
    let mut pairing = Pairing {
        held: held(history),
        hooks: runs_of("hook", "staged"),
        ci: runs_of("ci", "diff"),
        ancestry,
        paired: Paired::default(),
        seen: HashSet::new(),
    };
    pairing.paired.hook_runs = pairing.hooks.len();
    pairing.paired.ci_runs = pairing.ci.len();
    pairing.paired.paired_hooks = pairing.caught();
    pairing.paired.paired_ci = pairing.reached();
    pairing.paired
}

struct Pairing<'a> {
    held: Held<'a>,
    hooks: Vec<&'a Run>,
    ci: Vec<&'a Run>,
    ancestry: &'a dyn Ancestry,
    paired: Paired,
    /// A finding counts once for a rule, a fingerprint and a branch, in each reading.
    seen: HashSet<(&'static str, String, String, String)>,
}

impl<'a> Pairing<'a> {
    /// Counts the findings of each hook run that the earliest CI run after it does not hold, and
    /// returns how many hook runs had a CI run.
    fn caught(&mut self) -> usize {
        let mut paired_hooks = 0;
        for hook in self.hooks.clone() {
            let Some(after) = earliest(hook, &self.ci, self.ancestry) else {
                continue;
            };
            paired_hooks += 1;
            for slug in hook.rules.keys() {
                if comparable(slug, hook, after, &mut self.paired) {
                    self.count("caught", hook, after, slug);
                }
            }
        }
        paired_hooks
    }

    /// Counts the findings of each CI run that no hook run before it holds, and returns how many
    /// CI runs had a hook run before them.
    fn reached(&mut self) -> usize {
        let mut paired_ci = 0;
        for run in self.ci.clone() {
            let before: Vec<&Run> = self
                .hooks
                .iter()
                .copied()
                .filter(|hook| precedes(hook, run, &self.ci, self.ancestry))
                .collect();
            if before.is_empty() {
                continue;
            }
            paired_ci += 1;
            for slug in run.rules.keys() {
                let comparable: Vec<&Run> = before
                    .iter()
                    .copied()
                    .filter(|hook| comparable(slug, hook, run, &mut self.paired))
                    .collect();
                if !comparable.is_empty() {
                    self.count_new("reached", &comparable, run, slug);
                }
            }
        }
        paired_ci
    }

    /// Counts the findings of `hook` for `slug` that `after` does not hold.
    fn count(&mut self, reading: &'static str, hook: &Run, after: &Run, slug: &str) {
        let kept = held_by(&self.held, after, slug);
        let new: Vec<String> = held_by(&self.held, hook, slug)
            .into_iter()
            .filter(|fingerprint| !kept.contains(fingerprint))
            .map(str::to_owned)
            .collect();
        self.add(reading, hook, slug, new);
    }

    /// Counts the findings of `run` for `slug` that none of `hooks` holds.
    fn count_new(&mut self, reading: &'static str, hooks: &[&Run], run: &Run, slug: &str) {
        let holds: HashSet<&str> = hooks
            .iter()
            .flat_map(|hook| held_by(&self.held, hook, slug))
            .collect();
        let new: Vec<String> = held_by(&self.held, run, slug)
            .into_iter()
            .filter(|fingerprint| !holds.contains(fingerprint))
            .map(str::to_owned)
            .collect();
        self.add(reading, run, slug, new);
    }

    fn add(&mut self, reading: &'static str, run: &Run, slug: &str, fingerprints: Vec<String>) {
        for fingerprint in fingerprints {
            let key = (
                reading,
                slug.to_owned(),
                fingerprint,
                branch(run).to_owned(),
            );
            if !self.seen.insert(key) {
                continue;
            }
            let counts = match reading {
                "caught" => &mut self.paired.caught,
                _ => &mut self.paired.reached,
            };
            *counts.entry(slug.to_owned()).or_default() += 1;
        }
    }
}

/// A run with a branch to pair by; a writer records an empty branch as unset.
fn named(run: &Run) -> bool {
    run.branch.is_some()
}

fn branch(run: &Run) -> &str {
    run.branch.as_deref().unwrap_or_default()
}

fn held(history: &History) -> Held<'_> {
    let mut held: Held = HashMap::new();
    for finding in &history.findings {
        held.entry((finding.source, finding.run_id.as_str()))
            .or_default()
            .entry(finding.slug.as_str())
            .or_default()
            .insert(finding.fingerprint.as_str());
    }
    held
}

fn held_by<'a>(held: &'a Held<'a>, run: &Run, slug: &str) -> Vec<&'a str> {
    held.get(&(run.source, run.id.as_str()))
        .and_then(|rules| rules.get(slug))
        .map(|found| found.iter().copied().collect())
        .unwrap_or_default()
}

/// Whether `slug` takes part in the comparison of the two runs: it is in both with the same hash
/// that is not `null`. A rule that is in both and fails that is recorded as not comparable; a rule
/// absent from one of them did not run there, which is another case.
fn comparable(slug: &str, a: &Run, b: &Run, paired: &mut Paired) -> bool {
    let (Some(first), Some(second)) = (a.rules.get(slug), b.rules.get(slug)) else {
        return false;
    };
    let reason = match (&first.hash, &second.hash) {
        (Some(x), Some(y)) if x == y => return true,
        (Some(_), Some(_)) => Reason::Changed,
        _ => Reason::Unhashed,
    };
    paired
        .not_comparable
        .entry(slug.to_owned())
        .or_default()
        .insert(reason);
    false
}

/// The earliest CI run of the branch of `hook`, with the same `key_id`, that the hook run precedes.
fn earliest<'a>(hook: &Run, ci: &[&'a Run], ancestry: &dyn Ancestry) -> Option<&'a Run> {
    ci.iter()
        .copied()
        .filter(|run| precedes(hook, run, ci, ancestry))
        .min_by(|a, b| (a.started_at, &a.id).cmp(&(b.started_at, &b.id)))
}

/// Whether the hook run precedes the CI run (ADR 0019 decision 8): same branch and key, and the
/// `commit` of the hook run is a proper ancestor of the `head` of the CI run. When it is not, the
/// time of the runs decides, unless a CI run on a newer `head` started before the hook run.
fn precedes(hook: &Run, run: &Run, ci: &[&Run], ancestry: &dyn Ancestry) -> bool {
    if !named(hook) || hook.branch != run.branch || hook.key_id != run.key_id {
        return false;
    }
    let (commit, head) = (hook.commit.as_deref(), run.checked());
    if let (Some(commit), Some(head)) = (commit, head) {
        if commit == head {
            return false;
        }
        if ancestry.is_ancestor(commit, head) {
            return true;
        }
    }
    hook.started_at < run.started_at
        && !ci.iter().any(|other| {
            other.branch == run.branch
                && other.key_id == run.key_id
                && other.started_at < hook.started_at
                && match (head, other.checked()) {
                    (Some(head), Some(newer)) => ancestry.is_ancestor(head, newer),
                    _ => false,
                }
        })
}
