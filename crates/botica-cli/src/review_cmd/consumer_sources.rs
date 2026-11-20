use std::collections::BTreeSet;

use anyhow::{ensure, Result};
use botica_config::ReviewConfig;
use botica_core::review::coverage;
use serde_json::{json, Value};

use crate::review_cmd::tier;

pub(super) fn directory(assessment: &tier::Assessment, review: &ReviewConfig) -> Result<String> {
    if let Some(directory) = &review.dir {
        return Ok(directory.clone());
    }
    let files = assessment.git.base_paths()?;
    coverage::role_directory(None, |directory| {
        let prefix = format!("{directory}/");
        files.iter().any(|file| file.starts_with(&prefix))
    })
}

pub(super) fn bindings(
    assessment: &tier::Assessment,
    directory: &str,
    review: &ReviewConfig,
) -> Result<Value> {
    let names: BTreeSet<_> = review
        .expected_roles()
        .iter()
        .map(|role| role.name().to_owned())
        .collect();
    ensure!(
        names.len() == review.expected_roles().len(),
        "duplicate configured review roles"
    );
    let prefixes: Vec<_> = names
        .iter()
        .map(|name| format!("{directory}/{name}/"))
        .collect();
    let mut paths: BTreeSet<_> = assessment
        .git
        .base_paths()?
        .into_iter()
        .filter(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
        .collect();
    paths.insert(format!("{directory}/README.md"));
    paths.insert("skills-lock.json".to_owned());
    let mut files = Vec::new();
    for path in paths {
        let digest = assessment
            .git
            .blob(&path)?
            .map(|bytes| blake3::hash(&bytes).to_hex().to_string());
        files.push(json!({"path": path, "digest": digest}));
    }
    let mut inputs = assessment.report["inputs"].clone();
    inputs["roles_digest"] = json!(blake3::hash(&serde_json::to_vec(&files)?)
        .to_hex()
        .to_string());
    inputs["trusted_files"] = json!(files);
    inputs["risk_digest"] = json!(blake3::hash(&serde_json::to_vec(&assessment.report)?)
        .to_hex()
        .to_string());
    Ok(inputs)
}
