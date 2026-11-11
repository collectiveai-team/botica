//! Which hook run precedes which CI run (ADR 0019 decision 8).

use super::lines::Run;
use std::collections::HashMap;

/// The proper ancestors, among the commits of the runs, of each head that git answered for.
pub type Ancestors = HashMap<String, std::collections::HashSet<String>>;

/// Same branch and key, and the
/// `commit` of the hook run is a proper ancestor of the `head` of the CI run. When it is not, or
/// the clone lacks a commit, the time of the runs decides, unless a CI run on a newer `head`
/// started before the hook run.
pub struct Order<'a> {
    ancestors: Ancestors,
    /// The start of the earliest CI run of the same group whose `head` is a proper descendant of
    /// the `head` of each CI run, by the position of the run in `ci`.
    newer: Vec<Option<u64>>,
    ci: Vec<&'a Run>,
}

impl<'a> Order<'a> {
    pub fn new(ci: &[&'a Run], ancestors: Ancestors) -> Self {
        let mut earliest: HashMap<((&str, Option<&str>), &str), u64> = HashMap::new();
        for run in ci {
            let Some(found) = run.checked().and_then(|head| ancestors.get(head)) else {
                continue;
            };
            for older in found {
                let slot = earliest
                    .entry((group(run), older.as_str()))
                    .or_insert(u64::MAX);
                *slot = (*slot).min(run.started_at);
            }
        }
        let newer = ci
            .iter()
            .map(|run| {
                let head = run.checked()?;
                earliest.get(&(group(run), head)).copied()
            })
            .collect();
        Self {
            ancestors,
            newer,
            ci: ci.to_vec(),
        }
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
    pub fn precedes(&self, hook: &Run, position: usize) -> bool {
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
pub fn group(run: &Run) -> (&str, Option<&str>) {
    (
        run.branch.as_deref().unwrap_or_default(),
        run.key_id.as_deref(),
    )
}
