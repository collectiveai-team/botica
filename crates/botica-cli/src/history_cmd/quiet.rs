//! The readings that pair no hook run (ADR 0019 decision 8): the rules with no findings, the noise
//! of a rule and the noise of a role.

use super::lines::Run;
use super::read::History;
use std::collections::{BTreeMap, HashSet};

/// A rule that ran and found nothing in every run of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quiet {
    pub slug: String,
    pub runs: usize,
}

/// A rule whose findings a comment silences often.
#[derive(Debug, Clone, PartialEq)]
pub struct Noisy {
    pub slug: String,
    pub hits: u64,
    pub suppressed: u64,
}

impl Noisy {
    /// `suppressed` over `hits` plus `suppressed`.
    pub fn share(&self) -> f64 {
        self.suppressed as f64 / self.hits.saturating_add(self.suppressed) as f64
    }
}

/// A role whose findings `refute` judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub slug: String,
    pub judged: u64,
    pub dismissed: u64,
}

/// What counts as many suppressions.
#[derive(Debug, Clone, Copy)]
pub struct Noise {
    pub min_suppressed: u64,
    pub min_share: f64,
}

#[derive(Default)]
struct Totals {
    runs: usize,
    hits: u64,
    suppressed: u64,
    baselined: u64,
}

/// The rules with no findings and the rules with many suppressions, over `runs`: the runs of the
/// window that a trusted source wrote, with an origin that runs rules.
pub fn rules(runs: &[&Run], noise: Noise) -> (Vec<Quiet>, Vec<Noisy>) {
    let mut totals: BTreeMap<&str, Totals> = BTreeMap::new();
    for run in runs {
        for (slug, entry) in &run.rules {
            let total = totals.entry(slug).or_default();
            total.runs += 1;
            total.hits = total.hits.saturating_add(entry.hits);
            total.suppressed = total.suppressed.saturating_add(entry.suppressed);
            total.baselined = total.baselined.saturating_add(entry.baselined);
        }
    }
    let quiet = totals
        .iter()
        .filter(|(_, total)| total.hits == 0 && total.suppressed == 0 && total.baselined == 0)
        .map(|(slug, total)| Quiet {
            slug: (*slug).to_owned(),
            runs: total.runs,
        })
        .collect();
    let mut noisy: Vec<Noisy> = totals
        .iter()
        .map(|(slug, total)| Noisy {
            slug: (*slug).to_owned(),
            hits: total.hits,
            suppressed: total.suppressed,
        })
        .filter(|rule| rule.suppressed >= noise.min_suppressed.max(1))
        .filter(|rule| rule.share() >= noise.min_share)
        .collect();
    noisy.sort_by(|a, b| {
        b.share()
            .total_cmp(&a.share())
            .then_with(|| a.slug.cmp(&b.slug))
    });
    (quiet, noisy)
}

/// The findings of each role that `refute` judged and did not confirm, over those it judged.
/// `runs` holds the (source, run id) of the runs of the roles that count.
pub fn roles(history: &History, runs: &HashSet<(usize, &str)>) -> Vec<Role> {
    let mut by_role: BTreeMap<&str, Role> = BTreeMap::new();
    for finding in &history.findings {
        let Some(refutation) = finding.refutation.as_deref() else {
            continue;
        };
        if !runs.contains(&(finding.source, finding.run_id.as_str())) {
            continue;
        }
        let role = by_role.entry(&finding.slug).or_insert_with(|| Role {
            slug: finding.slug.clone(),
            judged: 0,
            dismissed: 0,
        });
        role.judged = role.judged.saturating_add(1);
        role.dismissed = role
            .dismissed
            .saturating_add(u64::from(refutation != "confirmed"));
    }
    by_role.into_values().collect()
}
