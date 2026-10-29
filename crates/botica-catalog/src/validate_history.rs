//! The checks of `history.toml`: the order of the releases, what each event changes, the renames,
//! and the reuse of a published code or id (ADR 0016).

use std::collections::{BTreeMap, BTreeSet};

use crate::issue::{issue, CatalogIssue};
use crate::model::{current_ids, Event, Rule, Status};

type Version = (u64, u64, u64);

/// The id each former id became, from [`current_ids`].
type Renames<'a> = BTreeMap<&'a str, &'a str>;

/// The last release whose history may give a code or an id to two rules: 0.13.0 and 0.16.0 did,
/// before ADR 0016.
const LAST_REUSE: Version = (0, 17, 1);

pub(crate) fn check_history(
    events: &[Event],
    rules: &[(String, Rule)],
    found: &mut Vec<CatalogIssue>,
) {
    let current = current_ids(events);
    let mut messages = Vec::new();
    for event in events {
        if !is_rule(rules, id_of(&current, &event.rule)) {
            messages.push(format!("`{}` is not a rule of the catalog", event.rule));
        }
        if event.status.is_none() && event.code.is_none() && event.renamed_from.is_none() {
            messages.push(format!(
                "the event of `{}` in `{}` has no status, code or renamed-from",
                event.rule, event.version
            ));
        }
    }
    check_order(events, &mut messages);
    check_changes(events, &current, &mut messages);
    check_renames(events, rules, &mut messages);
    check_reuse(events, rules, &current, &mut messages);
    found.extend(
        messages
            .into_iter()
            .map(|message| issue("history.toml", message)),
    );
}

fn id_of<'a>(current: &Renames<'a>, name: &'a str) -> &'a str {
    current.get(name).copied().unwrap_or(name)
}

fn is_rule(rules: &[(String, Rule)], id: &str) -> bool {
    rules.iter().any(|(_, rule)| rule.id == id)
}

fn parse_version(version: &str) -> Option<Version> {
    let parts: Vec<&str> = version.split('.').collect();
    let numbers: Vec<u64> = parts
        .iter()
        .filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|p| p.parse().ok())
        .collect();
    match (parts.len(), numbers.as_slice()) {
        (3, [major, minor, patch]) => Some((*major, *minor, *patch)),
        _ => None,
    }
}

/// Each version looks like a release, and none is older than the one of the event before.
fn check_order(events: &[Event], messages: &mut Vec<String>) {
    let mut last: Option<(Version, &str)> = None;
    for event in events {
        let Some(parsed) = parse_version(&event.version) else {
            messages.push(format!(
                "version `{}` must look like `0.1.0`",
                event.version
            ));
            continue;
        };
        if let Some((_, text)) = last.filter(|(previous, _)| parsed < *previous) {
            messages.push(format!(
                "version `{}` is older than `{text}`, the one of the event before: events are appended in release order",
                event.version
            ));
        }
        last = Some((parsed, &event.version));
    }
}

/// A status event changes the status of its rule, and a code event its code.
fn check_changes(events: &[Event], current: &Renames<'_>, messages: &mut Vec<String>) {
    let mut statuses: BTreeMap<&str, Status> = BTreeMap::new();
    let mut codes: BTreeMap<&str, &str> = BTreeMap::new();
    for event in events {
        let id = id_of(current, &event.rule);
        if let Some(status) = event.status {
            if statuses.insert(id, status) == Some(status) {
                messages.push(format!(
                    "`{}` is already {status:?} in `{}`: a status event has to change its status",
                    event.rule, event.version
                ));
            }
        }
        if let Some(code) = event.code.as_deref() {
            if codes.insert(id, code) == Some(code) {
                messages.push(format!(
                    "`{}` already has the code `{code}` in `{}`: a code event has to change its code",
                    event.rule, event.version
                ));
            }
        }
    }
}

/// A rename names an id that an event before it names and that no rule has, and the events after
/// it name the rule by its new id.
fn check_renames(events: &[Event], rules: &[(String, Rule)], messages: &mut Vec<String>) {
    let mut named: BTreeSet<&str> = BTreeSet::new();
    let mut renamed: BTreeMap<&str, &Event> = BTreeMap::new();
    for event in events {
        if let Some(rename) = renamed.get(event.rule.as_str()) {
            messages.push(format!(
                "`{}` is `{}` since `{}`: an event names the rule by the id it has at its version",
                event.rule, rename.rule, rename.version
            ));
        }
        if let Some(old) = event.renamed_from.as_deref() {
            if !named.contains(old) {
                messages.push(format!(
                    "`{}` is renamed from `{old}` in `{}`, and no event before names `{old}`",
                    event.rule, event.version
                ));
            }
            if is_rule(rules, old) {
                messages.push(format!(
                    "`{}` is renamed from `{old}` in `{}`, which is still the id of a rule",
                    event.rule, event.version
                ));
            }
            renamed.insert(old, event);
        }
        named.insert(&event.rule);
    }
}

/// Reports a code, an id or an alias that names two rules, unless the history gave it to both by
/// [`LAST_REUSE`].
fn check_reuse(
    events: &[Event],
    rules: &[(String, Rule)],
    current: &Renames<'_>,
    messages: &mut Vec<String>,
) {
    // For each name, each rule that it named and the first release that recorded it, if any. The
    // events are in release order, so the first one that names it is the first release.
    let mut owners: BTreeMap<&str, BTreeMap<&str, Option<Version>>> = BTreeMap::new();
    for event in events {
        let id = id_of(current, &event.rule);
        let version = parse_version(&event.version);
        let names = [
            Some(&event.rule),
            event.code.as_ref(),
            event.renamed_from.as_ref(),
        ];
        for name in names.into_iter().flatten() {
            owners.entry(name).or_default().entry(id).or_insert(version);
        }
    }
    for (_, rule) in rules {
        for name in [&rule.id, &rule.code].into_iter().chain(&rule.aliases) {
            owners
                .entry(name)
                .or_default()
                .entry(&rule.id)
                .or_insert(None);
        }
    }
    for (name, by_rule) in owners.iter().filter(|(_, by_rule)| by_rule.len() > 1) {
        let published = by_rule
            .values()
            .all(|first| first.is_some_and(|version| version <= LAST_REUSE));
        if !published {
            let names: Vec<String> = by_rule.keys().map(|id| format!("`{id}`")).collect();
            messages.push(format!(
                "`{name}` names {}: a code or an id that a release published never names another rule (ADR 0016)",
                names.join(" and ")
            ));
        }
    }
}
