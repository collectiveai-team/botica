//! What `npx skills add --json` writes to stdout: a JSON array with one entry for each skill that it
//! installed or skipped, or one entry that says why the call failed.

use std::collections::BTreeMap;

use serde_json::Value;

fn entries(stdout: &str) -> Vec<Value> {
    match serde_json::from_str::<Value>(stdout.trim()) {
        Ok(Value::Array(entries)) => entries,
        _ => Vec::new(),
    }
}

fn text<'a>(entry: &'a Value, key: &str) -> Option<&'a str> {
    entry.get(key).and_then(Value::as_str)
}

/// Why the call failed: the first line of the error of the first `failed` entry, followed by the
/// last `fatal:` or `error:` line below it when it has one.
pub fn failed_reason(stdout: &str) -> Option<String> {
    let entries = entries(stdout);
    let error = entries
        .iter()
        .find(|entry| text(entry, "status") == Some("failed"))
        .and_then(|entry| text(entry, "error"))?;
    let lines: Vec<&str> = error
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let headline = lines.first()?;
    let detail = lines
        .iter()
        .rev()
        .find(|line| line.starts_with("fatal:") || line.starts_with("error:"));
    Some(match detail {
        Some(detail) if detail != headline => format!("{headline}: {detail}"),
        _ => (*headline).to_owned(),
    })
}

/// The skills that the call skipped, by name, with the reason that it gives.
pub fn skipped(stdout: &str) -> BTreeMap<String, String> {
    entries(stdout)
        .iter()
        .filter(|entry| text(entry, "status") == Some("skipped"))
        .filter_map(|entry| {
            Some((
                text(entry, "name")?.to_owned(),
                text(entry, "reason")?.to_owned(),
            ))
        })
        .collect()
}
