//! The readings that pair a hook run with a CI run (ADR 0019 decision 8): "Caught before the
//! commit" and "Reached CI".

use super::ancestry::Ancestry;
use super::lines::Run;
use super::read::History;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// How many heads the pairing asks git about; a CI run past the limit is paired by time.
const MAX_HEADS: usize = 500;

/// Why a rule of two paired runs is left out of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// A run records `null` as the hash of the rule, so nothing says that the rule is the same.
    Unhashed,
    /// The two runs record different hashes: the version, the rule or its settings changed.
    Changed,
}

/// The two readings of the pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Reading {
    Caught,
    Reached,
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
    /// The heads that git was not asked about because of the limit.
    pub heads_unasked: usize,
}

/// The fingerprints of the findings of each rule in each run.
type Held<'a> = HashMap<(usize, &'a str), HashMap<&'a str, HashSet<&'a str>>>;

/// The proper ancestors, among the commits of the runs, of each head that git answered for.
type Ancestors = HashMap<String, HashSet<String>>;

pub fn pair(history: &History, window: &[&Run], ancestry: &dyn Ancestry) -> Paired {
    let runs_of = |origin: &str, mode: &str| -> Vec<&Run> {
        window
            .iter()
            .copied()
            .filter(|run| run.is(origin, mode) && named(run))
            .collect()
    };
    let hooks = runs_of("hook", "staged");
    let ci = runs_of("ci", "diff");
    let mut paired = Paired {
        hook_runs: hooks.len(),
        ci_runs: ci.len(),
        ..Paired::default()
    };
    let (ancestors, unasked) = ancestors(&hooks, &ci, ancestry);
    paired.heads_unasked = unasked;
    let order = Order::new(&ci, ancestors);
    let mut pairing = Pairing {
        held: held(history),
        hooks,
        ci,
        order,
        paired,
        seen: HashSet::new(),
    };
    pairing.paired.paired_hooks = pairing.caught();
    pairing.paired.paired_ci = pairing.reached();
    pairing.paired
}

/// Asks `ancestry` once for each distinct head of the CI runs, and returns the answers and how
/// many heads the limit left out.
fn ancestors(hooks: &[&Run], ci: &[&Run], ancestry: &dyn Ancestry) -> (Ancestors, usize) {
    let heads: BTreeSet<&str> = ci.iter().filter_map(|run| run.checked()).collect();
    let among: HashSet<&str> = hooks
        .iter()
        .filter_map(|run| run.commit.as_deref())
        .chain(heads.iter().copied())
        .collect();
    let mut answers = Ancestors::new();
    for head in heads.iter().take(MAX_HEADS) {
        if let Some(found) = ancestry.ancestors(head, &among) {
            answers.insert((*head).to_owned(), found);
        }
    }
    (answers, heads.len().saturating_sub(MAX_HEADS))
}

/// Which hook run precedes which CI run (ADR 0019 decision 8): same branch and key, and the
/// `commit` of the hook run is a proper ancestor of the `head` of the CI run. When it is not, or
/// the clone lacks a commit, the time of the runs decides, unless a CI run on a newer `head`
/// started before the hook run.
struct Order<'a> {
    ancestors: Ancestors,
    /// The start of the earliest CI run of the same branch and key whose `head` is a proper
    /// descendant of the `head` of each CI run, by the position of the run in `ci`.
    newer: Vec<Option<u64>>,
    ci: Vec<&'a Run>,
}

impl<'a> Order<'a> {
    fn new(ci: &[&'a Run], ancestors: Ancestors) -> Self {
        let mut order = Self {
            ancestors,
            newer: Vec::new(),
            ci: ci.to_vec(),
        };
        let mut by_group: HashMap<(&str, Option<&str>), Vec<usize>> = HashMap::new();
        for (position, run) in ci.iter().enumerate() {
            by_group.entry(group(run)).or_default().push(position);
        }
        order.newer = vec![None; ci.len()];
        for positions in by_group.values() {
            for &old in positions {
                order.newer[old] = positions
                    .iter()
                    .filter(|&&other| order.is_ancestor(ci[old].checked(), ci[other].checked()))
                    .map(|&other| ci[other].started_at)
                    .min();
            }
        }
        order
    }

    fn is_ancestor(&self, older: Option<&str>, newer: Option<&str>) -> bool {
        match (older, newer) {
            (Some(older), Some(newer)) if older != newer => self
                .ancestors
                .get(newer)
                .is_some_and(|found| found.contains(older)),
            _ => false,
        }
    }

    /// Whether `hook` precedes the CI run at `position` of `ci`.
    fn precedes(&self, hook: &Run, position: usize) -> bool {
        let run = self.ci[position];
        if group(hook) != group(run) {
            return false;
        }
        if let (Some(commit), Some(head)) = (hook.commit.as_deref(), run.checked()) {
            if commit == head {
                return false;
            }
            if self.is_ancestor(Some(commit), Some(head)) {
                return true;
            }
        }
        hook.started_at < run.started_at
            && self.newer[position].is_none_or(|at| at >= hook.started_at)
    }
}

/// What two runs share to be paired: the branch, and the key of their fingerprints.
fn group(run: &Run) -> (&str, Option<&str>) {
    (branch(run), run.key_id.as_deref())
}

struct Pairing<'a> {
    held: Held<'a>,
    hooks: Vec<&'a Run>,
    ci: Vec<&'a Run>,
    order: Order<'a>,
    paired: Paired,
    /// A finding counts once for a rule, a fingerprint and a branch, in each reading.
    seen: HashSet<(Reading, String, String, String)>,
}

impl<'a> Pairing<'a> {
    /// Counts the findings of each hook run that the earliest CI run after it does not hold, and
    /// returns how many hook runs had a CI run.
    fn caught(&mut self) -> usize {
        let mut paired_hooks = 0;
        for hook in self.hooks.clone() {
            let Some(after) = self.earliest(hook) else {
                continue;
            };
            paired_hooks += 1;
            for slug in hook.rules.keys() {
                if comparable(slug, hook, after, &mut self.paired) {
                    self.count(hook, &[after], slug);
                }
            }
        }
        paired_hooks
    }

    /// Counts the findings of each CI run that no hook run before it holds, and returns how many
    /// CI runs had a hook run before them.
    fn reached(&mut self) -> usize {
        let mut paired_ci = 0;
        for (position, run) in self.ci.clone().into_iter().enumerate() {
            let before: Vec<&Run> = self
                .hooks
                .iter()
                .copied()
                .filter(|hook| self.order.precedes(hook, position))
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
                    self.count(run, &comparable, slug);
                }
            }
        }
        paired_ci
    }

    /// The earliest CI run of the branch of `hook` that the hook run precedes.
    fn earliest(&self, hook: &Run) -> Option<&'a Run> {
        self.ci
            .iter()
            .enumerate()
            .filter(|(position, _)| self.order.precedes(hook, *position))
            .map(|(_, run)| *run)
            .min_by(|a, b| (a.started_at, &a.id).cmp(&(b.started_at, &b.id)))
    }

    /// Counts the findings of `run` for `slug` that none of `others` holds. The reading is
    /// `Caught` for the finding of a hook run against its CI run, and `Reached` for the finding of
    /// a CI run against the hook runs before it.
    fn count(&mut self, run: &Run, others: &[&Run], slug: &str) {
        let reading = if run.origin == "hook" {
            Reading::Caught
        } else {
            Reading::Reached
        };
        let holds: HashSet<&str> = others
            .iter()
            .flat_map(|other| held_by(&self.held, other, slug))
            .collect();
        let new: Vec<String> = held_by(&self.held, run, slug)
            .into_iter()
            .filter(|fingerprint| !holds.contains(fingerprint))
            .map(str::to_owned)
            .collect();
        for fingerprint in new {
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
                Reading::Caught => &mut self.paired.caught,
                Reading::Reached => &mut self.paired.reached,
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
