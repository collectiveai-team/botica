//! The readings that pair a hook run with a CI run (ADR 0019 decision 8): "Caught before the
//! commit" and "Reached CI".

use super::ancestry::{Ancestry, Commits};
use super::lines::Run;
use super::order::{group, Ancestors, Order};
use super::read::History;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// How many heads the pairing asks git about; a CI run past the limit is paired by time.
const MAX_HEADS: usize = 500;

/// How many hook runs and how many CI runs of one branch the pairing compares, the newest ones:
/// every hook run is compared with every CI run of its branch.
const MAX_PER_BRANCH: usize = 500;

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
    /// The older runs of a branch that the pairing left out because of the limit.
    pub runs_left_out: usize,
}

/// The fingerprints of the findings of each rule in each run.
type Held<'a> = HashMap<(usize, &'a str), HashMap<&'a str, HashSet<&'a str>>>;

pub fn pair(history: &History, window: &[&Run], since: u64, ancestry: &dyn Ancestry) -> Paired {
    let runs_of = |origin: &str, mode: &str| -> Vec<&Run> {
        window
            .iter()
            .copied()
            .filter(|run| run.is(origin, mode) && named(run))
            .collect()
    };
    let (hooks, left_hooks) = newest(runs_of("hook", "staged"));
    let (ci, left_ci) = newest(runs_of("ci", "diff"));
    let mut paired = Paired {
        hook_runs: hooks.len() + left_hooks,
        ci_runs: ci.len() + left_ci,
        runs_left_out: left_hooks + left_ci,
        ..Paired::default()
    };
    let (ancestors, unasked) = ancestors(&hooks, &ci, since, ancestry);
    paired.heads_unasked = unasked;
    let order = Order::new(&ci, ancestors);
    let held = held(history);
    let holders = holders(&hooks, &held);
    let mut pairing = Pairing {
        held,
        by_group: index(&hooks),
        holders,
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

/// The newest [`MAX_PER_BRANCH`] runs of each branch and key, and how many older ones are left out.
fn newest(mut runs: Vec<&Run>) -> (Vec<&Run>, usize) {
    runs.sort_by(|a, b| (b.started_at, &b.id).cmp(&(a.started_at, &a.id)));
    let mut kept_in: HashMap<(&str, Option<&str>), usize> = HashMap::new();
    let before = runs.len();
    runs.retain(|run| {
        let kept = kept_in.entry(group(run)).or_default();
        *kept += 1;
        *kept <= MAX_PER_BRANCH
    });
    let left = before - runs.len();
    (runs, left)
}

/// The positions of the hook runs that hold each fingerprint, by group and rule.
fn holders<'a>(hooks: &[&'a Run], held: &Held<'a>) -> HashMap<Holder<'a>, Vec<usize>> {
    let mut found: HashMap<Holder, Vec<usize>> = HashMap::new();
    for (position, hook) in hooks.iter().enumerate() {
        let Some(rules) = held.get(&(hook.source, hook.id.as_str())) else {
            continue;
        };
        for (slug, fingerprints) in rules {
            for fingerprint in fingerprints {
                found
                    .entry((group(hook), slug, fingerprint))
                    .or_default()
                    .push(position);
            }
        }
    }
    found
}

/// A group, a rule and a fingerprint.
type Holder<'a> = ((&'a str, Option<&'a str>), &'a str, &'a str);

/// The positions of `runs` by their group.
fn index<'a>(runs: &[&'a Run]) -> HashMap<(&'a str, Option<&'a str>), Vec<usize>> {
    let mut by_group: HashMap<(&str, Option<&str>), Vec<usize>> = HashMap::new();
    for (position, run) in runs.iter().enumerate() {
        by_group.entry(group(run)).or_default().push(position);
    }
    by_group
}

/// Asks `ancestry` once for the commits of the window, and keeps for each distinct head of the CI
/// runs the commits of the runs that are its proper ancestors. Returns that and how many heads the
/// limit left out.
fn ancestors(
    hooks: &[&Run],
    ci: &[&Run],
    since: u64,
    ancestry: &dyn Ancestry,
) -> (Ancestors, usize) {
    let heads: BTreeSet<&str> = ci.iter().filter_map(|run| run.checked()).collect();
    let among: HashSet<&str> = hooks
        .iter()
        .filter_map(|run| run.commit.as_deref())
        .chain(heads.iter().copied())
        .collect();
    let asked: Vec<&str> = heads.iter().copied().take(MAX_HEADS).collect();
    let commits: Commits = ancestry.commits(&asked, since);
    let answers = asked
        .iter()
        .filter(|head| commits.knows(head))
        .map(|head| ((*head).to_owned(), commits.ancestors(head, &among)))
        .collect();
    (answers, heads.len().saturating_sub(MAX_HEADS))
}

struct Pairing<'a> {
    held: Held<'a>,
    /// The positions in `hooks` of the hook runs of each group.
    by_group: HashMap<(&'a str, Option<&'a str>), Vec<usize>>,
    /// The positions in `hooks` of the hook runs of a group that hold a fingerprint of a rule.
    holders: HashMap<Holder<'a>, Vec<usize>>,
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
                    self.count(hook, after, slug);
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
            let before: Vec<usize> = self
                .by_group
                .get(&group(run))
                .map(|positions| {
                    positions
                        .iter()
                        .copied()
                        .filter(|&hook| self.order.precedes(self.hooks[hook], position))
                        .collect()
                })
                .unwrap_or_default();
            if before.is_empty() {
                continue;
            }
            paired_ci += 1;
            for slug in run.rules.keys() {
                let comparable: HashSet<usize> = before
                    .iter()
                    .copied()
                    .filter(|&hook| comparable(slug, self.hooks[hook], run, &mut self.paired))
                    .collect();
                if !comparable.is_empty() {
                    let new = self.unheld(run, slug, &comparable);
                    self.add(Reading::Reached, run, slug, new);
                }
            }
        }
        paired_ci
    }

    /// The fingerprints of `slug` in `run` that none of the hook runs at `comparable` holds.
    fn unheld(&self, run: &Run, slug: &str, comparable: &HashSet<usize>) -> Vec<String> {
        held_by(&self.held, run, slug)
            .into_iter()
            .filter(|fingerprint| {
                !self
                    .holders
                    .get(&(group(run), slug, *fingerprint))
                    .is_some_and(|holding| holding.iter().any(|hook| comparable.contains(hook)))
            })
            .map(str::to_owned)
            .collect()
    }

    /// The earliest CI run of the branch of `hook` that the hook run precedes.
    fn earliest(&self, hook: &Run) -> Option<&'a Run> {
        self.ci
            .iter()
            .enumerate()
            .filter(|(position, run)| {
                group(run) == group(hook) && self.order.precedes(hook, *position)
            })
            .map(|(_, run)| *run)
            .min_by(|a, b| (a.started_at, &a.id).cmp(&(b.started_at, &b.id)))
    }

    /// Counts the findings of the hook run `hook` for `slug` that `after` does not hold.
    fn count(&mut self, hook: &Run, after: &Run, slug: &str) {
        let holds: HashSet<&str> = held_by(&self.held, after, slug).into_iter().collect();
        let new: Vec<String> = held_by(&self.held, hook, slug)
            .into_iter()
            .filter(|fingerprint| !holds.contains(fingerprint))
            .map(str::to_owned)
            .collect();
        self.add(Reading::Caught, hook, slug, new);
    }

    /// Counts each fingerprint once for a reading, a rule and a branch.
    fn add(&mut self, reading: Reading, run: &Run, slug: &str, fingerprints: Vec<String>) {
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
