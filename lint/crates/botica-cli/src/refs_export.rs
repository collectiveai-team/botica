use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::{Origin, Reference, Topic};
use serde_json::{json, Value};

use crate::refs_collection::{label, Collection};

/// Prints every topic, built-in or of the project, as JSON, with the text that each file
/// reference shows: its lines, read from the project or from its repository at its commit. The
/// export stops at the first reference that cannot be read.
pub fn run(collection: &Collection) -> Result<ExitCode> {
    let mut topics = Vec::new();
    for topic in collection.loaded()?.topics() {
        topics.push(topic_json(collection, topic)?);
    }
    let text = serde_json::to_string_pretty(&json!({ "topics": topics }))?;
    writeln!(std::io::stdout().lock(), "{text}")?;
    Ok(ExitCode::SUCCESS)
}

fn topic_json(collection: &Collection, topic: &Topic) -> Result<Value> {
    let mut references = Vec::new();
    for reference in &topic.references {
        references.push(reference_json(collection, topic, reference)?);
    }
    let origin = match topic.origin {
        Origin::BuiltIn => "built-in",
        Origin::Project => "project",
    };
    Ok(json!({
        "id": topic.id,
        "title": topic.title,
        "origin": origin,
        "readme": topic.readme,
        "references": references,
    }))
}

fn reference_json(collection: &Collection, topic: &Topic, reference: &Reference) -> Result<Value> {
    let text = match &reference.path {
        Some(path) => {
            let file = collection.file_text(reference, path).map_err(|reason| {
                anyhow::anyhow!("topic `{}`, {}: {reason}", topic.id, label(reference))
            })?;
            Some(reference.excerpt(&file))
        }
        None => None,
    };
    Ok(json!({
        "repo": reference.repo,
        "rev": reference.rev,
        "license": reference.license,
        "path": reference.path,
        "url": reference.url,
        "lines": reference.lines,
        "anchor": reference.anchor,
        "note": reference.note,
        "text": text,
    }))
}
