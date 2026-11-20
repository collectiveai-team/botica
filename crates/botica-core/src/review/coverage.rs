//! The generated `<!-- botica:covered -->` block of a role: the hook and job references it binds,
//! and how the block is rendered and found, without filesystem access.

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};

use super::roles::{yaml, Covered};

pub const OPEN: &str = "<!-- botica:covered -->";
pub const CLOSE: &str = "<!-- /botica:covered -->";

/// The hook and job references of `covered`, each with the blake3 digest of its whole source file.
/// `read` returns the bytes of a repository-relative path, or `None` when the file is missing.
pub fn check_references(
    covered: &Covered,
    read: impl Fn(&str) -> Result<Option<Vec<u8>>>,
) -> Result<Vec<Value>> {
    let mut references = Vec::new();
    for (kind, listed) in [("hook", &covered.hooks), ("job", &covered.jobs)] {
        for declaration in listed {
            let bytes = read(&declaration.file)?
                .with_context(|| format!("missing trusted coverage source {}", declaration.file))?;
            let config = parse(&declaration.file, &bytes)?;
            let found = if kind == "hook" {
                hook_present(&config, &declaration.id)
            } else {
                job_present(&config, &declaration.id)
            };
            ensure!(
                found,
                "missing trusted {kind} {} in {}",
                declaration.id,
                declaration.file
            );
            references.push(
                json!({"kind": kind, "file": declaration.file, "id": declaration.id,
                "digest": blake3::hash(&bytes).to_hex().to_string()}),
            );
        }
    }
    Ok(references)
}

/// The block for `references`, sorted by kind, file and id. Fails on a duplicate declaration.
pub fn render(references: &mut [Value]) -> Result<String> {
    references.sort_by_key(|reference| {
        format!(
            "{}\0{}\0{}",
            reference["kind"], reference["file"], reference["id"]
        )
    });
    ensure!(
        references.windows(2).all(|pair| pair[0] != pair[1]),
        "duplicate coverage declarations"
    );
    let mut lines = vec![OPEN.to_owned()];
    for reference in references.iter() {
        lines.push(format!(
            "- {} {} in {}: blake3 {}",
            reference["kind"].as_str().unwrap_or(""),
            reference["id"],
            reference["file"],
            reference["digest"].as_str().unwrap_or("")
        ));
    }
    lines.push(CLOSE.to_owned());
    Ok(lines.join("\n"))
}

/// The block that `text` holds between its markers, with CRLF line ends read as LF, or `None`
/// when it has no markers.
pub fn current_block(text: &str) -> Result<Option<String>> {
    let text = text.replace("\r\n", "\n");
    ensure!(
        text.matches(OPEN).count() <= 1 && text.matches(CLOSE).count() <= 1,
        "duplicate coverage markers"
    );
    match (text.find(OPEN), text.find(CLOSE)) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) if start < end => {
            Ok(Some(text[start..end + CLOSE.len()].to_owned()))
        }
        _ => bail!("unbalanced coverage markers"),
    }
}

fn parse(file: &str, bytes: &[u8]) -> Result<Value> {
    let text = std::str::from_utf8(bytes)?;
    if file.ends_with(".toml") {
        Ok(serde_json::to_value(toml::from_str::<toml::Value>(text)?)?)
    } else {
        yaml(text)
    }
}

fn hook_present(config: &Value, id: &str) -> bool {
    config["repos"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|repo| repo["hooks"].as_array().into_iter().flatten())
        .any(|hook| hook["id"] == id)
}

fn job_present(config: &Value, id: &str) -> bool {
    if let Some(jobs) = config.get("jobs") {
        return jobs.get(id).is_some_and(Value::is_object);
    }
    config.get(id).is_some_and(|job| {
        job.is_object()
            && ["script", "trigger", "extends"]
                .iter()
                .any(|key| job.get(key).is_some())
    })
}
