//! What the report says: the readings of ADR 0019 decision 8 over the runs of a window.

use super::ancestry::Ancestry;
use super::lines::Run;
use super::pairs::{self, Paired};
use super::quiet::{self, Noise, Noisy, Quiet, Role};
use super::read::{History, LOCAL};
use std::collections::{BTreeMap, HashSet};

const DAY: u64 = 24 * 60 * 60;

/// The runs that the window held, by where they came from and whether they were read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub local: usize,
    pub ci_trusted: usize,
    pub ci_untrusted: usize,
    /// The runs that fed the rules with no findings and the noise: trusted, of an origin that runs
    /// rules.
    pub rule_runs: usize,
    /// The runs whose mode or origin this reader does not know, by their mode and origin.
    pub unknown: BTreeMap<String, usize>,
}

pub struct Report {
    pub days: u64,
    pub counts: Counts,
    pub skipped_lines: usize,
    pub notes: Vec<String>,
    pub paired: Paired,
    pub quiet: Vec<Quiet>,
    pub noisy: Vec<Noisy>,
    pub roles: Vec<Role>,
    pub noise: Noise,
}

/// The first second of a window of `days` days that ends at `now`.
pub fn since(now: u64, days: u64) -> u64 {
    now.saturating_sub(days.saturating_mul(DAY))
}

/// The report of the last `days` days up to `now`, in Unix seconds.
pub fn build(
    history: &History,
    now: u64,
    days: u64,
    noise: Noise,
    ancestry: &dyn Ancestry,
) -> Report {
    let since = since(now, days);
    let window: Vec<&Run> = history
        .runs
        .iter()
        .filter(|run| run.started_at >= since)
        .collect();
    let mut counts = Counts::default();
    let mut known = Vec::new();
    for run in window {
        if !run.is_known() {
            *counts
                .unknown
                .entry(format!("mode {}, origin {}", run.mode, run.origin))
                .or_default() += 1;
            continue;
        }
        match (run.source == LOCAL, history.trusted(run.source)) {
            (true, _) => counts.local += 1,
            (false, true) => counts.ci_trusted += 1,
            (false, false) => counts.ci_untrusted += 1,
        }
        known.push(run);
    }
    let trusted: Vec<&Run> = known
        .iter()
        .copied()
        .filter(|run| history.trusted(run.source))
        .collect();
    let of_rules: Vec<&Run> = trusted
        .iter()
        .copied()
        .filter(|run| !is_role(run) && run.origin != "classify")
        .collect();
    counts.rule_runs = of_rules.len();
    let (quiet, noisy) = quiet::rules(&of_rules, noise);
    let role_runs: HashSet<(usize, &str)> = trusted
        .iter()
        .filter(|run| is_role(run))
        .map(|run| (run.source, run.id.as_str()))
        .collect();
    let roles = quiet::roles(history, &role_runs);
    let paired_window: Vec<&Run> = known
        .iter()
        .copied()
        .filter(|run| run.origin != "hook" || run.source == LOCAL)
        .collect();
    Report {
        days,
        counts,
        skipped_lines: history.skipped_lines,
        notes: history.notes.clone(),
        paired: pairs::pair(history, &paired_window, since, ancestry),
        quiet,
        noisy,
        roles,
        noise,
    }
}

fn is_role(run: &Run) -> bool {
    run.origin.starts_with("review:")
}
