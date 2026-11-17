use anyhow::{ensure, Context, Result};
use botica_core::review::roles::{yaml, RoleDefinition};
use serde_json::{json, Value};

use super::super::tier;
#[path = "consumer_rules.rs"]
mod rules;

const OPEN: &str = "<!-- botica:covered -->";
const CLOSE: &str = "<!-- /botica:covered -->";

fn check_file(assessment: &tier::Assessment, file: &str) -> Result<(Value, String)> {
    let bytes = assessment
        .git
        .blob(file)?
        .with_context(|| format!("missing trusted coverage source {file}"))?;
    let text = std::str::from_utf8(&bytes)?;
    let parsed = if file.ends_with(".toml") {
        serde_json::to_value(toml::from_str::<toml::Value>(text)?)?
    } else {
        yaml(text)?
    };
    Ok((parsed, blake3::hash(&bytes).to_hex().to_string()))
}

fn hook_present(config: &Value, id: &str) -> bool {
    config["repos"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|repo| repo["hooks"].as_array().into_iter().flatten())
        .any(|hook| hook["id"] == id)
}

fn current_block(text: &str) -> Result<Option<String>> {
    ensure!(
        text.matches(OPEN).count() <= 1 && text.matches(CLOSE).count() <= 1,
        "duplicate coverage markers"
    );
    match (text.find(OPEN), text.find(CLOSE)) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) if start < end => {
            Ok(Some(text[start..end + CLOSE.len()].to_owned()))
        }
        _ => anyhow::bail!("unbalanced coverage markers"),
    }
}

pub(super) fn inspect(
    assessment: &tier::Assessment,
    role: &RoleDefinition,
    text: &str,
) -> Result<Value> {
    let declarations = role
        .covered
        .as_ref()
        .context("unverified legacy coverage: declare covered metadata")?;
    let mut references = rules::references(assessment, &declarations.rules)?;
    for (kind, listed) in [("hook", &declarations.hooks), ("job", &declarations.jobs)] {
        for declaration in listed {
            let (config, digest) = check_file(assessment, &declaration.file)?;
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
            references.push(json!({"kind": kind, "file": declaration.file, "id": declaration.id, "digest": digest}));
        }
    }
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
    for reference in &references {
        lines.push(format!(
            "- {} {} in {}: blake3 {}",
            reference["kind"].as_str().unwrap_or(""),
            reference["id"],
            reference["file"],
            reference["digest"].as_str().unwrap_or("")
        ));
    }
    lines.push(CLOSE.to_owned());
    let expected = lines.join("\n");
    let normalized = text.replace("\r\n", "\n");
    let actual = current_block(&normalized)?;
    Ok(
        json!({"status": if actual.as_deref() == Some(&expected) { "current" } else { "stale" },
        "expected_block": expected, "references": references}),
    )
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
