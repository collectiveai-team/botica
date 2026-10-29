//! What the releases did to the rules, read from `history.toml` (ADR 0016).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::Serialize;

use crate::load::Catalog;
use crate::model::{current_ids, Event, Status};

/// A version of botica, `0.13.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Release {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Release {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl FromStr for Release {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let numbers: Vec<u64> = text
            .split('.')
            .filter(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            .filter_map(|part| part.parse().ok())
            .collect();
        match (text.split('.').count(), numbers.as_slice()) {
            (3, [major, minor, patch]) => Ok(Self::new(*major, *minor, *patch)),
            _ => Err(format!("`{text}` is not a version like `0.13.0`")),
        }
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// One thing that a release did to a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub version: String,
    /// The id of the rule at `version`.
    pub rule: String,
    /// The code of the rule at `version`; `None` while no release has recorded one.
    pub code: Option<String>,
    #[serde(flatten)]
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "change", rename_all = "kebab-case")]
pub enum ChangeKind {
    Added {
        status: Status,
    },
    Status {
        from: Option<Status>,
        to: Status,
    },
    /// `replaced_by` is the rule whose `replaces` names the removed one.
    Removed {
        replaced_by: Option<String>,
    },
    Renamed {
        from: String,
    },
    Code {
        from: Option<String>,
        to: String,
    },
}

/// The status and the code that the events before one gave its rule.
#[derive(Default)]
struct Known<'a> {
    seen: bool,
    status: Option<Status>,
    code: Option<&'a str>,
}

impl Catalog {
    /// What the releases after `after`, up to `up_to`, did to the rules, in release order.
    pub fn changes(&self, after: Release, up_to: Release) -> Vec<Change> {
        let history = self.history();
        let current = current_ids(history);
        let mut known: BTreeMap<&str, Known<'_>> = BTreeMap::new();
        let mut changes = Vec::new();
        for event in history {
            let id = current
                .get(event.rule.as_str())
                .copied()
                .unwrap_or(&event.rule);
            let before = known.entry(id).or_default();
            let kinds = self.kinds(event, id, before);
            before.seen = true;
            before.status = event.status.or(before.status);
            before.code = event.code.as_deref().or(before.code);
            let in_range = event
                .version
                .parse::<Release>()
                .is_ok_and(|version| after < version && version <= up_to);
            if in_range {
                changes.extend(kinds.into_iter().map(|kind| Change {
                    version: event.version.clone(),
                    rule: event.rule.clone(),
                    code: before.code.map(str::to_owned),
                    kind,
                }));
            }
        }
        changes
    }

    fn kinds(&self, event: &Event, id: &str, before: &Known<'_>) -> Vec<ChangeKind> {
        let mut kinds = Vec::new();
        match event.status {
            Some(status) if !before.seen => kinds.push(ChangeKind::Added { status }),
            Some(Status::Removed) => kinds.push(ChangeKind::Removed {
                replaced_by: self.replacement(id),
            }),
            Some(to) => kinds.push(ChangeKind::Status {
                from: before.status,
                to,
            }),
            None => {}
        }
        if let Some(from) = &event.renamed_from {
            kinds.push(ChangeKind::Renamed { from: from.clone() });
        }
        if let Some(to) = event.code.as_deref().filter(|_| before.seen) {
            kinds.push(ChangeKind::Code {
                from: before.code.map(str::to_owned),
                to: to.to_owned(),
            });
        }
        kinds
    }

    fn replacement(&self, id: &str) -> Option<String> {
        self.rules()
            .iter()
            .find(|rule| rule.replaces.as_deref() == Some(id))
            .map(|rule| rule.id.clone())
    }
}
