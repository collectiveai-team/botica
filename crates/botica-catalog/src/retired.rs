//! The retired names of the rules: the codes and the ids that the history gave a rule and that it
//! no longer has (ADR 0016, decision 2).

use std::collections::BTreeMap;

use crate::model::{current_ids, Event, Rule};

/// What a retired name was, for the rule that has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetiredKind {
    Code,
    Id,
}

/// A code or an id that a release gave to `rule` and that it lost in `since`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retired {
    /// The id the rule has in the catalog.
    pub rule: String,
    pub kind: RetiredKind,
    /// The release that gave the rule another code or id.
    pub since: String,
}

/// The retired names of `rules`, from `events`. A name that a rule of the catalog has as its id,
/// its code or an alias of another rule is not retired: it names that rule. A name that two rules
/// lost names the last one that lost it.
pub(crate) fn retired_names(events: &[Event], rules: &[Rule]) -> BTreeMap<String, Retired> {
    let current = current_ids(events);
    let mut last_code: BTreeMap<&str, &str> = BTreeMap::new();
    let mut found: BTreeMap<String, Retired> = BTreeMap::new();
    for event in events {
        let rule = current
            .get(event.rule.as_str())
            .copied()
            .unwrap_or(&event.rule);
        let mut retire = |name: &str, kind: RetiredKind| {
            found.insert(
                name.to_owned(),
                Retired {
                    rule: rule.to_owned(),
                    kind,
                    since: event.version.clone(),
                },
            );
        };
        if let Some(old) = &event.renamed_from {
            retire(old, RetiredKind::Id);
        }
        if let Some(code) = event.code.as_deref() {
            if let Some(old) = last_code.insert(rule, code).filter(|old| *old != code) {
                retire(old, RetiredKind::Code);
            }
        }
    }
    found.retain(|name, retired| {
        rules.iter().all(|other| {
            let owns = other.id == *name || other.code == *name || other.aliases.contains(name);
            !owns || (other.id == retired.rule && other.aliases.contains(name))
        })
    });
    found
}
