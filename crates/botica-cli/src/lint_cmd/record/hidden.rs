//! The findings that a run produced and does not report, which the history counts (ADR 0019
//! decision 3): the ones that a suppression channel silenced and the ones that the baseline hid.

use crate::project;
use botica_config::Resolution;
use botica_core::{postprocess, Diagnostic};
use std::collections::BTreeMap;

#[derive(Default)]
pub(in crate::lint_cmd) struct Hidden {
    /// The findings that a comment silenced.
    pub silenced: Vec<Diagnostic>,
    /// The findings that the baseline hid.
    pub baselined: Vec<Diagnostic>,
    /// The extension rules whose processes all finished.
    pub extensions_finished: Vec<String>,
    /// The rules of the findings of a `--ext` command that failed.
    pub failed_rules: Vec<String>,
}

impl Hidden {
    pub(in crate::lint_cmd) fn new(silenced: Vec<Diagnostic>) -> Self {
        Self {
            silenced,
            ..Self::default()
        }
    }

    /// Adds what the extension rules brought: the rules whose processes all finished, and the
    /// findings that the comments silenced.
    pub(in crate::lint_cmd) fn add_extensions(
        &mut self,
        finished: Vec<String>,
        silenced: Vec<Diagnostic>,
    ) {
        self.extensions_finished = finished;
        self.silenced.extend(silenced);
    }

    /// Keeps the hidden findings that `keeps` says the run reports, as the filter of `--diff` does
    /// for the findings themselves.
    pub(in crate::lint_cmd) fn keep_only(&mut self, keeps: impl Fn(&Diagnostic) -> bool) {
        self.silenced.retain(&keeps);
        self.baselined.retain(&keeps);
    }

    /// Applies the configuration to the findings that a comment silenced, as it applies to the
    /// ones that the run reports: a rule that the configuration does not apply at a path produced
    /// no finding there, and a file reached twice counts its findings once.
    pub(super) fn settle(&mut self, resolution: &Resolution, prefix: Option<&str>) {
        let silenced = std::mem::take(&mut self.silenced);
        self.silenced = postprocess::apply(silenced, project::verdicts(resolution, prefix));
    }
}

/// How many of `found` each rule has.
pub(super) fn by_rule(found: &[Diagnostic]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for diagnostic in found {
        *counts.entry(diagnostic.rule.clone()).or_default() += 1;
    }
    counts
}
