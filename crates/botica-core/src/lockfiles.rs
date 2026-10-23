//! `lockfile-committed`: each manifest that declares dependencies has the lockfile of one package
//! manager next to it or in a folder above it (the root of its workspace), each root module of
//! Terraform has its `.terraform.lock.hcl` in its own folder, and the walk of the project reaches
//! that lockfile, so `.gitignore` does not leave it out.

use std::collections::{BTreeMap, BTreeSet};

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

/// Whether the rule reads the text of `path`: a manifest of an ecosystem or a file of Terraform.
pub fn reads(path: &str) -> bool {
    let name = name_of(path);
    is_terraform(name)
        || ECOSYSTEMS
            .iter()
            .any(|ecosystem| ecosystem.manifest == name)
}

const TERRAFORM_LOCKFILE: &str = ".terraform.lock.hcl";

/// A file of the Terraform language: OpenTofu reads `.tofu` files too.
fn is_terraform(name: &str) -> bool {
    name.ends_with(".tf") || name.ends_with(".tofu")
}

/// What the files of one folder of Terraform hold.
#[derive(Default)]
struct Module<'a> {
    /// The first file that makes the folder a root module: it has a `provider`, a `backend` or a
    /// `cloud` block, which only a root module carries.
    root: Option<&'a str>,
    /// Whether a file has a `provider` or a `required_providers` block, so `init` locks providers.
    providers: bool,
}

/// The first word of `line`, and whether a block or an attribute follows it.
fn keyword(line: &str) -> Option<&str> {
    let end = line
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(line.len());
    let rest = line[end..].trim_start();
    (end > 0 && rest.starts_with(['"', '{', '='])).then(|| &line[..end])
}

/// The braces that `line` opens and closes, outside its strings and its comment.
fn braces(line: &str) -> (usize, usize) {
    let (mut opened, mut closed, mut quoted, mut escaped) = (0, 0, false, false);
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match (escaped, c) {
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                _ => escaped = false,
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '#' => break,
            '/' if chars.peek() == Some(&'/') => break,
            '{' => opened += 1,
            '}' => closed += 1,
            _ => {}
        }
    }
    (opened, closed)
}

/// The terminator of the heredoc that `line` opens (`<<EOT` or `<<-EOT`).
fn heredoc(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("<<")?;
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The folder that a local `source` of a `module` block names, from the root of the project.
fn local_module(dir: &str, value: &str) -> Option<String> {
    let value = value.trim().strip_prefix('"')?.split('"').next()?;
    if !(value.starts_with("./") || value.starts_with("../")) {
        return None;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// The local folder that the `source = "..."` attribute `line` names.
fn source_of(dir: &str, line: &str) -> Option<String> {
    let (_, value) = line.split_once('=')?;
    local_module(dir, value)
}

/// Reads one file of Terraform into the module of its folder, and adds to `called` the folders
/// that its `module` blocks name in a local `source`.
fn scan<'a>(source: &Source<'a>, module: &mut Module<'a>, called: &mut BTreeSet<String>) {
    let dir = dir_of(source.path);
    let (mut depth, mut block, mut until) = (0_usize, "", None::<String>);
    for line in source.text.lines() {
        let line = line.trim();
        if let Some(end) = &until {
            if line == end {
                until = None;
            }
            continue;
        }
        match (depth, block, keyword(line)) {
            (0, _, Some("provider")) => {
                module.root.get_or_insert(source.path);
                module.providers = true;
            }
            (1, "terraform", Some("required_providers")) => module.providers = true,
            (1, "terraform", Some("backend" | "cloud")) => {
                module.root.get_or_insert(source.path);
            }
            (1, "module", Some("source")) => called.extend(source_of(dir, line)),
            (0, _, Some("module")) => {
                let body = line
                    .split_once('{')
                    .map_or("", |(_, body)| body.trim_start());
                if keyword(body) == Some("source") {
                    called.extend(source_of(dir, body));
                }
            }
            _ => {}
        }
        let (opened, closed) = braces(line);
        if depth == 0 && opened > closed {
            block = keyword(line).unwrap_or("");
        }
        depth = (depth + opened).saturating_sub(closed);
        until = heredoc(line);
    }
}

/// The root modules of Terraform with providers and no `.terraform.lock.hcl` in their folder. A
/// folder that a `module` block of the project calls is a child module: its providers are locked
/// by the root module that calls it.
fn terraform(
    paths: &BTreeSet<&str>,
    sources: &[Source<'_>],
    severity: Severity,
) -> Vec<Diagnostic> {
    let mut modules: BTreeMap<&str, Module<'_>> = BTreeMap::new();
    let mut called = BTreeSet::new();
    for source in sources
        .iter()
        .filter(|source| is_terraform(name_of(source.path)))
    {
        let module = modules.entry(dir_of(source.path)).or_default();
        scan(source, module, &mut called);
    }
    modules
        .into_iter()
        .filter(|(dir, module)| {
            module.providers
                && !called.contains(*dir)
                && !paths.contains(join(dir, TERRAFORM_LOCKFILE).as_str())
        })
        .filter_map(|(dir, module)| {
            let folder = if dir.is_empty() { "." } else { dir };
            Some(Diagnostic::file_level(
                module.root?,
                LOCKFILE_COMMITTED,
                severity,
                format!(
                    "No `{TERRAFORM_LOCKFILE}` pins the providers of the root module `{folder}`: \
                     run `terraform init` or `tofu init` in that folder, commit the lockfile it \
                     writes, and take it out of `.gitignore` if it is there"
                ),
            ))
        })
        .collect()
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
/// managers, and the root modules of Terraform with no lockfile. `paths` are every file a walk of
/// the project reaches.
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
    found.extend(terraform(&paths, sources, severity));
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}
