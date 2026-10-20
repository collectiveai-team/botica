//! `lockfile-committed`: each manifest that declares dependencies has the lockfile of one package
//! manager next to it or in a folder above it (the root of its workspace), and the walk of the
//! project reaches that lockfile, so `.gitignore` does not leave it out.

use std::collections::BTreeSet;

use crate::diagnostic::{Diagnostic, Severity};

pub const LOCKFILE_COMMITTED: &str = "lockfile-committed";

/// A manifest of the project, from its root, and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

struct Ecosystem {
    manifest: &'static str,
    /// Each lockfile of the ecosystem, with the package manager that writes it.
    lockfiles: &'static [(&'static str, &'static str)],
    /// Whether the manifest declares something that a lockfile pins.
    declares: fn(&str) -> bool,
}

const ECOSYSTEMS: [Ecosystem; 4] = [
    Ecosystem {
        manifest: "pyproject.toml",
        lockfiles: &[
            ("uv.lock", "uv"),
            ("poetry.lock", "poetry"),
            ("pdm.lock", "pdm"),
            ("Pipfile.lock", "pipenv"),
        ],
        declares: python_declares,
    },
    Ecosystem {
        manifest: "package.json",
        lockfiles: &[
            ("pnpm-lock.yaml", "pnpm"),
            ("package-lock.json", "npm"),
            ("npm-shrinkwrap.json", "npm"),
            ("yarn.lock", "yarn"),
            ("bun.lock", "bun"),
            ("bun.lockb", "bun"),
        ],
        declares: node_declares,
    },
    Ecosystem {
        manifest: "go.mod",
        lockfiles: &[("go.sum", "go")],
        declares: go_declares,
    },
    Ecosystem {
        manifest: "Cargo.toml",
        lockfiles: &[("Cargo.lock", "cargo")],
        declares: cargo_declares,
    },
];

/// A project of Python: `pyproject.toml` with a `[project]` table.
fn python_declares(text: &str) -> bool {
    text.parse::<toml::Table>()
        .is_ok_and(|table| table.get("project").is_some_and(toml::Value::is_table))
}

const NODE_DEPENDENCIES: [&str; 3] = ["dependencies", "devDependencies", "optionalDependencies"];

fn node_declares(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok_and(|manifest| {
        NODE_DEPENDENCIES.iter().any(|key| {
            manifest
                .get(key)
                .and_then(serde_json::Value::as_object)
                .is_some_and(|dependencies| !dependencies.is_empty())
        })
    })
}

/// A module of Go with a `require` directive: one with no dependency has no `go.sum`.
fn go_declares(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.strip_prefix("require")
            .is_some_and(|rest| rest.starts_with([' ', '\t', '(']))
    })
}

/// A package or a workspace of Cargo.
fn cargo_declares(text: &str) -> bool {
    text.parse::<toml::Table>()
        .is_ok_and(|table| table.contains_key("package") || table.contains_key("workspace"))
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// Whether the rule reads the text of `path`: a manifest of an ecosystem.
pub fn reads(path: &str) -> bool {
    let name = name_of(path);
    ECOSYSTEMS
        .iter()
        .any(|ecosystem| ecosystem.manifest == name)
}

/// The package manager that `packageManager` of a `package.json` names (`pnpm@9.1.0` is `pnpm`).
fn named_manager(text: &str) -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(text).ok()?;
    let field = manifest.get("packageManager")?.as_str()?;
    Some(field.split('@').next()?.to_owned())
}

/// The lockfiles of `ecosystem` in `dir`, each with its manager.
fn lockfiles_in<'e>(
    ecosystem: &'e Ecosystem,
    dir: &str,
    paths: &BTreeSet<&str>,
) -> Vec<(&'e str, &'e str)> {
    ecosystem
        .lockfiles
        .iter()
        .filter(|(file, _)| paths.contains(join(dir, file).as_str()))
        .copied()
        .collect()
}

/// The lockfiles of `ecosystem` in the nearest folder from `dir` up to the root that holds one.
fn nearest<'e>(
    ecosystem: &'e Ecosystem,
    dir: &str,
    paths: &BTreeSet<&str>,
) -> Vec<(&'e str, &'e str)> {
    let mut dir = dir;
    loop {
        let found = lockfiles_in(ecosystem, dir, paths);
        if !found.is_empty() || dir.is_empty() {
            return found;
        }
        dir = dir_of(dir);
    }
}

/// What the rule says of one manifest, or `None` when its lockfile is in order.
fn problem(ecosystem: &Ecosystem, source: &Source<'_>, paths: &BTreeSet<&str>) -> Option<String> {
    let dir = dir_of(source.path);
    let here = lockfiles_in(ecosystem, dir, paths);
    let managers: BTreeSet<&str> = here.iter().map(|(_, manager)| *manager).collect();
    if managers.len() > 1 {
        let files: Vec<String> = here.iter().map(|(file, _)| format!("`{file}`")).collect();
        return Some(format!(
            "`{}` has the lockfiles of {} package managers ({}): keep the one of the manager the \
             project uses and delete the others",
            source.path,
            managers.len(),
            files.join(", ")
        ));
    }
    let found = nearest(ecosystem, dir, paths);
    let named = (ecosystem.manifest == "package.json")
        .then(|| named_manager(source.text))
        .flatten();
    if found.is_empty() {
        let expected = named
            .as_deref()
            .and_then(|manager| {
                ecosystem
                    .lockfiles
                    .iter()
                    .find(|(_, of)| *of == manager)
                    .map(|(file, _)| *file)
            })
            .unwrap_or(ecosystem.lockfiles[0].0);
        return Some(format!(
            "No lockfile pins the dependencies of `{}`: commit the `{expected}` that its package \
             manager writes, next to it or at the root of its workspace, and take it out of \
             `.gitignore` if it is there",
            source.path
        ));
    }
    let named = named?;
    if found.iter().any(|(_, manager)| *manager == named) {
        return None;
    }
    Some(format!(
        "`{}` names `{named}` in `packageManager`, and its lockfile is `{}`: install with `{named}` \
         and delete the lockfile of the other manager, or change `packageManager`",
        source.path, found[0].0
    ))
}

/// The manifests of `sources` whose lockfile is missing, or that have the lockfiles of two
/// managers. `paths` are every file a walk of the project reaches.
pub fn check(paths: &[String], sources: &[Source<'_>], severity: Severity) -> Vec<Diagnostic> {
    let paths: BTreeSet<&str> = paths.iter().map(String::as_str).collect();
    let mut found: Vec<Diagnostic> = sources
        .iter()
        .filter_map(|source| {
            let name = name_of(source.path);
            let ecosystem = ECOSYSTEMS.iter().find(|e| e.manifest == name)?;
            if !(ecosystem.declares)(source.text) {
                return None;
            }
            let message = problem(ecosystem, source, &paths)?;
            Some(Diagnostic::file_level(
                source.path,
                LOCKFILE_COMMITTED,
                severity,
                message,
            ))
        })
        .collect();
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}
