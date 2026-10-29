//! `botica rules --since <version>`: what the releases after a version, up to this binary, did to
//! the rules of the built-in catalog (ADR 0016).

use std::io::Write;

use anyhow::Result;
use botica_catalog::{Catalog, Change, ChangeKind, Release};
use serde::Serialize;

use crate::explain_cmd::status_name;

/// Version of the JSON document of `botica rules --since --json`.
const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct ChangesDocument {
    schema_version: u32,
    since: String,
    until: String,
    changes: Vec<Change>,
}

pub fn run(since: Release, json: bool) -> Result<()> {
    let until: Release = env!("CARGO_PKG_VERSION")
        .parse()
        .map_err(anyhow::Error::msg)?;
    let changes = Catalog::embedded()?.changes(since, until);
    let mut out = std::io::stdout().lock();
    if json {
        let document = ChangesDocument {
            schema_version: SCHEMA_VERSION,
            since: since.to_string(),
            until: until.to_string(),
            changes,
        };
        serde_json::to_writer(&mut out, &document)?;
        writeln!(out)?;
    } else {
        for change in &changes {
            writeln!(out, "{}", line(change))?;
        }
    }
    Ok(())
}

fn line(change: &Change) -> String {
    let code = change
        .code
        .as_ref()
        .map(|code| format!(" ({code})"))
        .unwrap_or_default();
    format!(
        "{:<8}{}{code}  {}",
        change.version,
        change.rule,
        what(&change.kind)
    )
}

fn what(kind: &ChangeKind) -> String {
    match kind {
        ChangeKind::Added { status } => format!("added as {}", status_name(*status)),
        ChangeKind::Status {
            from: Some(from),
            to,
        } => {
            format!("{} -> {}", status_name(*from), status_name(*to))
        }
        ChangeKind::Status { from: None, to } => format!("now {}", status_name(*to)),
        ChangeKind::Removed {
            replaced_by: Some(rule),
        } => format!("removed, replaced by {rule}"),
        ChangeKind::Removed { replaced_by: None } => "removed".to_owned(),
        ChangeKind::Renamed { from } => format!("renamed from {from}"),
        ChangeKind::Code {
            from: Some(from),
            to,
        } => format!("code {from} -> {to}"),
        ChangeKind::Code { from: None, to } => format!("code {to}"),
    }
}
