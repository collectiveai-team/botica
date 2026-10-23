//! What `botica recipe install` does for the tools: one `uv add` for each group, for the packages
//! that `pyproject.toml` does not declare, and one `pnpm add` or `npm install` for the Node
//! packages that `package.json` does not declare.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use botica_catalog::{is_node_manager, ResolvedTool, DEFAULT_TOOL_GROUP, NODE_MANAGERS};

use crate::recipes_install_run::failure;
use crate::recipes_install_steps::Outcome;
use crate::recipes_package_json::{self as package_json, PackageJson};
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

/// Runs `uv add` for each group of the `missing` tools, and again for each package when a call for
/// several packages fails: the label and the failure of the call that settled each package.
fn add_missing<'a>(
    root: &Path,
    missing: &[&'a ResolvedTool],
) -> BTreeMap<(&'a str, &'a str), (String, Option<String>)> {
    let mut attempts = BTreeMap::new();
    for (group, packages) in calls(missing) {
        let (label, reason) = add(root, group, &packages);
        if reason.is_some() && packages.len() > 1 {
            for package in &packages {
                attempts.insert((group, *package), add(root, group, &[package]));
            }
        } else {
            for package in &packages {
                attempts.insert((group, *package), (label.clone(), reason.clone()));
            }
        }
    }
    attempts
}

/// Adds the packages of uv that are missing and says, for each one in order, whether it was there,
/// was added, or is still missing after the call.
fn install_uv(root: &Path, tools: &[&ResolvedTool]) -> Vec<Outcome> {
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
        .map(|(tool, _)| *tool)
        .collect();
    let attempts = add_missing(root, &missing);
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

/// Runs `pnpm add --save-dev` or `npm install --save-dev` in the root of the project: why it
/// failed, when it did.
fn add_node(root: &Path, manager: &str, packages: &[&str]) -> (String, Option<String>) {
    let verb = if manager == "pnpm" { "add" } else { "install" };
    let label = format!("`{manager} {verb} --save-dev`");
    let mut command = Command::new(manager);
    command
        .args([verb, "--save-dev"])
        .args(packages)
        .current_dir(root);
    let reason = failure(&mut command, &label);
    (label, reason)
}

/// Adds the Node packages that are missing with the manager that the project uses, and says, for
/// each one in order, whether it was there, was added, or is still missing after the call. A
/// project that uses another manager than pnpm or npm leaves each missing package pending.
fn install_node(root: &Path, tools: &[&ResolvedTool]) -> Vec<Outcome> {
    let Some(first) = tools.first() else {
        return Vec::new();
    };
    let before = PackageJson::read(root);
    let there: Vec<Option<String>> = tools
        .iter()
        .map(|tool| before.problem(&tool.manager, &tool.package))
        .collect();
    if matches!(before, PackageJson::Missing | PackageJson::Unusable(_)) {
        return there
            .into_iter()
            .map(|reason| Outcome::Pending(reason.unwrap_or_default()))
            .collect();
    }
    let manager = package_json::manager(root, &first.manager);
    let missing: Vec<&str> = tools
        .iter()
        .zip(&there)
        .filter(|(_, problem)| problem.is_some())
        .map(|(tool, _)| tool.package.as_str())
        .collect();
    if !NODE_MANAGERS.contains(&manager.as_str()) {
        return there
            .into_iter()
            .map(|problem| match problem {
                None => Outcome::Present,
                Some(_) => Outcome::Pending(format!(
                    "the project uses {manager}: add the package with it as a dev dependency"
                )),
            })
            .collect();
    }
    let attempts = add_missing_node(root, &manager, &missing);
    let after = PackageJson::read(root);
    tools
        .iter()
        .zip(there)
        .map(|(tool, problem)| {
            if problem.is_none() {
                Outcome::Present
            } else if after.problem(&tool.manager, &tool.package).is_none() {
                Outcome::Added(None)
            } else {
                let (label, reason) = &attempts[tool.package.as_str()];
                Outcome::Failed(
                    reason
                        .clone()
                        .unwrap_or_else(|| format!("not in package.json after {label}")),
                )
            }
        })
        .collect()
}

/// Adds `missing` with `manager` in one call, and again one by one when that call fails for several
/// packages: the label of the call and its failure, for each package.
fn add_missing_node<'a>(
    root: &Path,
    manager: &str,
    missing: &[&'a str],
) -> BTreeMap<&'a str, (String, Option<String>)> {
    let mut attempts = BTreeMap::new();
    if missing.is_empty() {
        return attempts;
    }
    let (label, reason) = add_node(root, manager, missing);
    for package in missing.iter().copied() {
        let attempt = if reason.is_some() && missing.len() > 1 {
            add_node(root, manager, &[package])
        } else {
            (label.clone(), reason.clone())
        };
        attempts.insert(package, attempt);
    }
    attempts
}

/// Adds the packages that are missing and says, for each package in order, whether it was there,
/// was added, or is still missing after the call. A call that fails for several packages is made
/// again for each package, so that one that does not exist does not fail the others.
pub fn install(root: &Path, tools: &[ResolvedTool]) -> Vec<Outcome> {
    let (node, uv): (Vec<&ResolvedTool>, Vec<&ResolvedTool>) = tools
        .iter()
        .partition(|tool| is_node_manager(&tool.manager));
    let mut uv = install_uv(root, &uv).into_iter();
    let mut node = install_node(root, &node).into_iter();
    tools
        .iter()
        .map(|tool| {
            let next = if is_node_manager(&tool.manager) {
                node.next()
            } else {
                uv.next()
            };
            next.expect("one outcome for each tool")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(manager: &str, package: &str) -> ResolvedTool {
        ResolvedTool {
            manager: manager.to_owned(),
            group: "dev".to_owned(),
            package: package.to_owned(),
            from: "mine".to_owned(),
        }
    }

    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "botica-install-tools-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create the folder");
        dir
    }

    #[test]
    fn a_pyproject_that_cannot_be_used_leaves_every_package_pending_and_runs_nothing() {
        let dir = folder("pyproject");
        std::fs::write(dir.join("pyproject.toml"), "= broken").expect("write");

        let outcomes = install(&dir, &[tool("uv", "ruff"), tool("uv", "pytest")]);

        let texts: Vec<String> = outcomes.iter().map(Outcome::text).collect();
        assert_eq!(
            texts,
            [
                "pending: pyproject.toml is not valid TOML",
                "pending: pyproject.toml is not valid TOML"
            ]
        );
    }

    #[test]
    fn a_project_without_package_json_leaves_the_node_packages_pending_in_their_place() {
        let dir = folder("no-package-json");
        std::fs::write(
            dir.join("pyproject.toml"),
            "[dependency-groups]\ndev = [\"ruff\"]\n",
        )
        .expect("write");

        let outcomes = install(&dir, &[tool("pnpm", "knip"), tool("uv", "ruff")]);

        let texts: Vec<String> = outcomes.iter().map(Outcome::text).collect();
        assert_eq!(texts, ["pending: no package.json", "ok"]);
    }

    #[test]
    fn a_project_of_yarn_leaves_the_missing_packages_pending_and_runs_nothing() {
        let dir = folder("yarn");
        std::fs::write(
            dir.join("package.json"),
            r#"{"devDependencies": {"typescript": "^7"}}"#,
        )
        .expect("write");
        std::fs::write(dir.join("yarn.lock"), "").expect("write");

        let outcomes = install(&dir, &[tool("pnpm", "typescript"), tool("pnpm", "knip")]);

        let texts: Vec<String> = outcomes.iter().map(Outcome::text).collect();
        assert_eq!(
            texts,
            [
                "ok",
                "pending: the project uses yarn: add the package with it as a dev dependency"
            ]
        );
    }
}
