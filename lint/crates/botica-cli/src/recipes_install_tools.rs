//! What `botica recipe install` does for the tools: one `uv add` for each group, for the packages
//! that `pyproject.toml` does not declare.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use botica_catalog::{ResolvedTool, DEFAULT_TOOL_GROUP};

use crate::recipes_install_run::failure;
use crate::recipes_install_steps::Outcome;
use crate::recipes_pyproject::Pyproject;

/// The packages to ask for in each group, with the groups in the order they first appear.
fn calls<'a>(missing: &[&'a ResolvedTool]) -> Vec<(&'a str, Vec<&'a str>)> {
    let mut calls: Vec<(&str, Vec<&str>)> = Vec::new();
    for tool in missing {
        match calls.iter_mut().find(|(group, _)| *group == tool.group) {
            Some((_, packages)) => packages.push(&tool.package),
            None => calls.push((&tool.group, vec![&tool.package])),
        }
    }
    calls
}

/// The arguments of `uv add` that choose the group.
fn group_flags(group: &str) -> Vec<&str> {
    if group == DEFAULT_TOOL_GROUP {
        vec!["--dev"]
    } else {
        vec!["--group", group]
    }
}

/// Runs `uv add` in the root of the project: why it failed, when it did.
fn add(root: &Path, group: &str, packages: &[&str]) -> (String, Option<String>) {
    let flags = group_flags(group);
    let label = format!("`uv add {}`", flags.join(" "));
    let mut command = Command::new("uv");
    command
        .arg("add")
        .args(&flags)
        .args(packages)
        .current_dir(root);
    let reason = failure(&mut command, &label);
    (label, reason)
}

/// Adds the packages that are missing and says, for each package in order, whether it was there,
/// was added, or is still missing after the call. A call that fails for a group of several packages
/// is made again for each package, so that one that does not exist does not fail the others.
pub fn install(root: &Path, tools: &[ResolvedTool]) -> Vec<Outcome> {
    let before = Pyproject::read(root);
    let there: Vec<Option<String>> = tools
        .iter()
        .map(|tool| before.problem(&tool.package))
        .collect();
    if matches!(before, Pyproject::Missing | Pyproject::Unusable(_)) {
        return there
            .into_iter()
            .map(|reason| Outcome::Pending(reason.unwrap_or_default()))
            .collect();
    }
    let missing: Vec<&ResolvedTool> = tools
        .iter()
        .zip(&there)
        .filter(|(_, problem)| problem.is_some())
        .map(|(tool, _)| tool)
        .collect();
    let mut attempts: BTreeMap<(&str, &str), (String, Option<String>)> = BTreeMap::new();
    for (group, packages) in calls(&missing) {
        let (label, reason) = add(root, group, &packages);
        if reason.is_some() && packages.len() > 1 {
            for package in &packages {
                attempts.insert((group, package), add(root, group, &[package]));
            }
        } else {
            for package in &packages {
                attempts.insert((group, package), (label.clone(), reason.clone()));
            }
        }
    }
    let after = Pyproject::read(root);
    tools
        .iter()
        .zip(there)
        .map(|(tool, problem)| {
            if problem.is_none() {
                Outcome::Present
            } else if after.problem(&tool.package).is_none() {
                Outcome::Added(None)
            } else {
                let (label, reason) = &attempts[&(tool.group.as_str(), tool.package.as_str())];
                Outcome::Failed(
                    reason
                        .clone()
                        .unwrap_or_else(|| format!("not in pyproject.toml after {label}")),
                )
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(package: &str) -> ResolvedTool {
        ResolvedTool {
            manager: "uv".to_owned(),
            group: "dev".to_owned(),
            package: package.to_owned(),
            from: "mine".to_owned(),
        }
    }

    #[test]
    fn a_pyproject_that_cannot_be_used_leaves_every_package_pending_and_runs_nothing() {
        let dir = std::env::temp_dir().join(format!("botica-install-tools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the folder");
        std::fs::write(dir.join("pyproject.toml"), "= broken").expect("write");

        let outcomes = install(&dir, &[tool("ruff"), tool("pytest")]);

        let texts: Vec<String> = outcomes.iter().map(Outcome::text).collect();
        assert_eq!(
            texts,
            [
                "pending: pyproject.toml is not valid TOML",
                "pending: pyproject.toml is not valid TOML"
            ]
        );
    }
}
