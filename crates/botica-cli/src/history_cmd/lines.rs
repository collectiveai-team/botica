//! The lines of the history, checked as ADR 0019 decision 9.4 states: a line that fails a check
//! is skipped, and nothing that a line holds is run.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

type Object = Map<String, Value>;

const SEVERITIES: [&str; 8] = ["error", "warning", "info", "hint", "P0", "P1", "P2", "P3"];
const REFUTATIONS: [&str; 5] = [
    "confirmed",
    "pre-existing",
    "unreachable",
    "not-reproduced",
    "duplicate",
];
const DETAIL_MAX: usize = 64 * 1024;

/// The `mode` values that ADR 0019 decision 6 defines.
pub const MODES: [&str; 9] = [
    "full", "paths", "diff", "staged", "message", "judged", "reviewed", "mutants", "classify",
];

/// What a run says of one rule that it evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub hits: u64,
    pub suppressed: u64,
    pub baselined: u64,
    pub hash: Option<String>,
}

/// A line of `runs.jsonl`, with the source that it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub source: usize,
    pub id: String,
    pub started_at: u64,
    pub commit: Option<String>,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub key_id: Option<String>,
    pub mode: String,
    pub origin: String,
    pub rules: BTreeMap<String, Entry>,
    /// The blake3 of the line, so that two lines of one run that differ in any key are told apart.
    pub raw: [u8; 32],
}

impl Run {
    /// Whether the mode and the origin are values that this reader knows; a run with another
    /// value enters none of the readings.
    pub fn is_known(&self) -> bool {
        MODES.contains(&self.mode.as_str())
            && (matches!(self.origin.as_str(), "ci" | "hook" | "cli" | "classify")
                || self.origin.starts_with("review:"))
    }

    /// Whether a person's hook or a CI job started the run in this mode.
    pub fn is(&self, origin: &str, mode: &str) -> bool {
        self.origin == origin && self.mode == mode
    }

    /// The commit that a CI run checked: `head`, or `commit` when there is none.
    pub fn checked(&self) -> Option<&str> {
        self.head.as_deref().or(self.commit.as_deref())
    }
}

/// A line of `findings.jsonl`: the keys that the readings use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub source: usize,
    pub run_id: String,
    pub slug: String,
    pub fingerprint: String,
    pub occurrence: u64,
    pub refutation: Option<String>,
}

/// The run that `line` holds, or `None` when the line is skipped: not a JSON object, of an
/// unknown `v`, or with a value that fails its check.
pub fn run(source: usize, line: &[u8]) -> Option<Run> {
    let map = object(line)?;
    version(&map)?;
    let id = hex(&map, "run_id", 16)?;
    let started_at = map.get("started_at")?.as_u64()?;
    let commit = optional(&map, "commit", |value| hex_of(value, &[40, 64]))?;
    let head = optional(&map, "head", |value| hex_of(value, &[40, 64]))?;
    let branch =
        optional(&map, "branch", |value| plain(value, 256))?.filter(|name| !name.is_empty());
    let key_id = optional(&map, "key_id", |value| hex_of(value, &[8]))?;
    optional(&map, "rules_fingerprint", |value| hex_of(value, &[64]))?;
    optional(&map, "detail", |value| detail(value).then_some(()))?;
    let mode = label(map.get("mode")?)?;
    let origin = label(map.get("origin")?)?;
    let rules = rules(map.get("rules")?)?;
    Some(Run {
        source,
        id,
        started_at,
        commit,
        head,
        branch,
        key_id,
        mode,
        origin,
        rules,
        raw: *blake3::hash(line).as_bytes(),
    })
}

/// The finding that `line` holds, or `None` when the line is skipped.
pub fn finding(source: usize, line: &[u8]) -> Option<Finding> {
    let map = object(line)?;
    version(&map)?;
    let run_id = hex(&map, "run_id", 16)?;
    optional(&map, "code", |value| plain(value, 256))?;
    let slug = plain(map.get("slug")?, 256)?;
    let severity = map.get("severity")?.as_str()?;
    SEVERITIES.contains(&severity).then_some(())?;
    relative(map.get("path")?.as_str()?)?;
    map.get("line")?.as_u64()?;
    let fingerprint = hex(&map, "fingerprint", 16)?;
    let occurrence = map.get("occurrence")?.as_u64()?;
    optional(&map, "ref", |value| plain(value, 256))?;
    let refutation = optional(&map, "refutation", |value| {
        let text = value.as_str()?;
        REFUTATIONS.contains(&text).then(|| text.to_owned())
    })?;
    Some(Finding {
        source,
        run_id,
        slug,
        fingerprint,
        occurrence,
        refutation,
    })
}

fn object(line: &[u8]) -> Option<Object> {
    match serde_json::from_slice(line).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// `v` is a non-negative integer, and the one that this reader knows.
fn version(map: &Object) -> Option<()> {
    (map.get("v")?.as_u64()? == 1).then_some(())
}

/// The value of `key`, `Some(None)` when it is absent or `null`, and `None` when it fails `check`.
fn optional<T>(map: &Object, key: &str, check: impl Fn(&Value) -> Option<T>) -> Option<Option<T>> {
    match map.get(key) {
        None | Some(Value::Null) => Some(None),
        Some(value) => check(value).map(Some),
    }
}

fn hex(map: &Object, key: &str, len: usize) -> Option<String> {
    hex_of(map.get(key)?, &[len])
}

fn hex_of(value: &Value, lens: &[usize]) -> Option<String> {
    let text = value.as_str()?;
    (lens.contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| text.to_owned())
}

/// A string of at most `max` bytes with no control character.
fn plain(value: &Value, max: usize) -> Option<String> {
    let text = value.as_str()?;
    is_plain(text, max).then(|| text.to_owned())
}

fn is_plain(text: &str, max: usize) -> bool {
    text.len() <= max && !text.chars().any(is_unsafe)
}

/// A control character, or a format character that reorders or hides the text around it (the
/// marks and overrides of bidirectional text, the zero-width characters and the byte order mark).
fn is_unsafe(found: char) -> bool {
    found.is_control()
        || matches!(
            found,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
        )
}

/// A `mode` or an `origin`: at most 64 bytes of `[a-z:-]`.
fn label(value: &Value) -> Option<String> {
    let text = value.as_str()?;
    let fits = text.len() <= 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b':' || byte == b'-');
    fits.then(|| text.to_owned())
}

/// A path relative to the root of the project: no `..` component and no control character.
fn relative(path: &str) -> Option<()> {
    let outside = path.starts_with(['/', '\\'])
        || path.split(['/', '\\']).any(|part| part == "..")
        || path.chars().any(is_unsafe);
    (!outside).then_some(())
}

/// An object of at most 64 KB whose strings have no control character.
fn detail(value: &Value) -> bool {
    value.is_object()
        && serde_json::to_vec(value).is_ok_and(|bytes| bytes.len() <= DETAIL_MAX)
        && no_control(value)
}

fn no_control(value: &Value) -> bool {
    match value {
        Value::String(text) => !text.chars().any(is_unsafe),
        Value::Array(items) => items.iter().all(no_control),
        Value::Object(map) => map
            .iter()
            .all(|(key, item)| !key.chars().any(is_unsafe) && no_control(item)),
        _ => true,
    }
}

fn rules(value: &Value) -> Option<BTreeMap<String, Entry>> {
    value
        .as_object()?
        .iter()
        .map(|(slug, entry)| Some((slug.clone(), entry_of(slug, entry)?)))
        .collect()
}

fn entry_of(slug: &str, value: &Value) -> Option<Entry> {
    if !is_plain(slug, 256) {
        return None;
    }
    let map = value.as_object()?;
    Some(Entry {
        hits: map.get("hits")?.as_u64()?,
        suppressed: map.get("suppressed")?.as_u64()?,
        baselined: map.get("baselined")?.as_u64()?,
        hash: optional(map, "hash", |value| hex_of(value, &[16]))?,
    })
}
