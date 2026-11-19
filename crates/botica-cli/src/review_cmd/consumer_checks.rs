use std::path::Path;

use anyhow::{ensure, Result};
use botica_config::PathMatcher;
use botica_core::review::roles::RoleDefinition;
use serde_json::{json, Value};

pub(super) fn unmatched(role: &RoleDefinition, paths: &[String]) -> Vec<Value> {
    role.applies.paths.iter().filter(|pattern| {
        PathMatcher::new(&[(*pattern).clone()]).is_ok_and(|matcher| !paths.iter().any(|path| matcher.is_match(path)))
    }).map(|pattern| json!({"code": "unmatched-role-path", "role": role.role,
        "message": format!("applies.paths glob {pattern:?} matches no path at the pinned destination or head")})).collect()
}

pub(super) fn skill(root: &Path, role: &str) -> Result<Value> {
    let relative = format!(".agents/skills/review-{role}/SKILL.md");
    let mut path = root.to_path_buf();
    for component in relative.split('/') {
        path.push(component);
        ensure!(
            !std::fs::symlink_metadata(&path)?.file_type().is_symlink(),
            "installed skill path is a symbolic link"
        );
    }
    ensure!(
        std::fs::metadata(&path)?.is_file(),
        "installed skill is not a regular file"
    );
    let bytes = std::fs::read(path)?;
    ensure!(!bytes.is_empty(), "installed skill is empty");
    Ok(
        json!({"status": "available", "path": relative, "digest": blake3::hash(&bytes).to_hex().to_string()}),
    )
}
