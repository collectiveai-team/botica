use std::collections::BTreeSet;

use serde::Deserialize;

/// `[[skills]]`: skills of a repository that `botica recipe install` installs with `npx skills add`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillStep {
    /// A repository, with `#<tag>` to pin it.
    pub source: String,
    pub names: Vec<String>,
}

/// `[[files]]`: a file that `botica recipe install` adds from a template when it does not exist.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileStep {
    /// From the root of the project.
    pub path: String,
    /// A file of the folder of the recipes.
    pub template: String,
}

/// `[[sections]]`: a section of a file, which starts with its marker.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionStep {
    /// From the root of the project.
    pub path: String,
    pub marker: String,
    /// A file of the folder of the recipes.
    pub template: String,
}

/// `[[links]]`: a symbolic link that `botica recipe install` adds when its path does not exist.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkStep {
    /// From the root of the project.
    pub path: String,
    /// What the link points to, as the text of the link: relative to the folder of the link.
    pub target: String,
}

/// The package managers that a `tools` step can name.
pub const TOOL_MANAGERS: [&str; 3] = ["uv", "pnpm", "npm"];

/// The managers of Node packages, whose packages `package.json` declares.
pub const NODE_MANAGERS: [&str; 2] = ["pnpm", "npm"];

/// The group that a `tools` step adds to when it names none.
pub const DEFAULT_TOOL_GROUP: &str = "dev";

/// `[[tools]]`: packages that `botica recipe install` adds to the project with a package manager
/// when `pyproject.toml` (uv) or `package.json` (pnpm, npm) does not declare them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolStep {
    /// One of `TOOL_MANAGERS`.
    pub manager: String,
    /// The group of dependencies the packages go to; `DEFAULT_TOOL_GROUP` when absent.
    pub group: Option<String>,
    /// Requirements: for uv a name, optionally with extras, version specifiers and a marker; for
    /// pnpm and npm a name, optionally scoped, and optionally `@` and a range.
    pub packages: Vec<String>,
}

/// The one line that `unignore` can name: the manifest of skills (ADR 0006, CES-107).
pub const UNIGNORE_WHITELIST: [&str; 1] = ["skills-lock.json"];

/// `[[gitignore]]`: lines that `botica recipe install` adds to `.gitignore` when they are missing
/// and lines of the whitelist that it removes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitignoreStep {
    #[serde(default)]
    pub lines: Vec<String>,
    /// Literal lines to remove, only the ones of `UNIGNORE_WHITELIST`.
    #[serde(default)]
    pub unignore: Vec<String>,
}

/// What is wrong with the steps of one recipe, one message for each problem.
pub(crate) fn step_problems(
    skills: &[SkillStep],
    files: &[FileStep],
    sections: &[SectionStep],
    links: &[LinkStep],
    gitignore: &[GitignoreStep],
    tools: &[ToolStep],
) -> Vec<String> {
    let mut problems = skill_problems(skills);
    problems.extend(file_problems(files));
    problems.extend(section_problems(sections));
    problems.extend(link_problems(links));
    problems.extend(gitignore_problems(gitignore));
    problems.extend(tool_problems(tools));
    problems
}

/// Whether `name` starts with a letter or a digit and has only letters, digits, `.`, `_` and `-`.
fn is_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The name at the start of a requirement, and the rest: its extras, versions and marker.
fn split_requirement(requirement: &str) -> (&str, &str) {
    let end = requirement
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .unwrap_or(requirement.len());
    requirement.split_at(end)
}

/// Whether `requirement` is a package name, optionally followed by extras, version specifiers and
/// a marker, as `blockbuster>=1.5,<1.6` or `pytest-memray ; sys_platform != 'win32'`. A direct
/// reference (`name @ url`) is not one: the packages of a recipe come from the index. `uv add`
/// checks the rest.
fn is_requirement(requirement: &str) -> bool {
    let (name, rest) = split_requirement(requirement);
    let rest = rest.trim_start();
    requirement == requirement.trim()
        && is_name(name)
        && !rest.contains(['@', '\n', '\r'])
        && (rest.is_empty() || rest.starts_with(['[', '<', '>', '=', '!', '~', ';']))
}

/// The normalized name of the package of a requirement: `my-pkg` for `My_Pkg>=1`.
pub fn requirement_package(requirement: &str) -> String {
    normalized_package(split_requirement(requirement.trim_start()).0)
}

/// Whether two requirements of `manager` ask for the same: the same package, and the same extras,
/// versions and marker once the white space is dropped. `ruff` and `ruff>=0.5` are different
/// requirements, and so are `knip` and `knip@^6`.
pub(crate) fn same_requirement(manager: &str, a: &str, b: &str) -> bool {
    if is_node_manager(manager) {
        return a == b;
    }
    let rest = |requirement: &str| -> String {
        split_requirement(requirement.trim_start())
            .1
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect()
    };
    requirement_package(a) == requirement_package(b) && rest(a) == rest(b)
}

/// Whether `manager` installs Node packages.
pub fn is_node_manager(manager: &str) -> bool {
    NODE_MANAGERS.contains(&manager)
}

/// The name of a Node requirement and the rest, which starts with the `@` of its range:
/// `@biomejs/biome` and `@^2` for `@biomejs/biome@^2`.
fn split_node_requirement(requirement: &str) -> (&str, &str) {
    let scope = usize::from(requirement.starts_with('@'));
    match requirement[scope..].find('@') {
        Some(at) => requirement.split_at(scope + at),
        None => (requirement, ""),
    }
}

/// Whether `part` is a part of a name of the npm registry: lower case letters, digits, `.`, `_`,
/// `~` and `-`, starting with a letter or a digit.
fn is_node_name_part(part: &str) -> bool {
    part.chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && part.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '~' | '-')
        })
}

/// Whether `requirement` is a name of the npm registry, optionally scoped (`@biomejs/biome`) and
/// optionally followed by `@` and a range (`knip@^6`). A tag, a URL, a path or an alias
/// (`npm:`) is not one: the range has no `:` and no `/`.
fn is_node_requirement(requirement: &str) -> bool {
    let (name, rest) = split_node_requirement(requirement);
    let name_ok = match name.strip_prefix('@') {
        Some(scoped) => scoped
            .split_once('/')
            .is_some_and(|(scope, bare)| is_node_name_part(scope) && is_node_name_part(bare)),
        None => is_node_name_part(name),
    };
    let range = rest.strip_prefix('@');
    name_ok
        && name.len() <= 214
        && (rest.is_empty()
            || range.is_some_and(|range| {
                !range.trim().is_empty()
                    && range == range.trim()
                    && !range.contains([':', '/', '\n', '\r'])
            }))
}

/// The package of a requirement of `manager`, as the manager compares it: `my-pkg` for `My_Pkg>=1`
/// with uv, `@biomejs/biome` for `@biomejs/biome@^2` with pnpm or npm.
pub fn tool_package(manager: &str, requirement: &str) -> String {
    if is_node_manager(manager) {
        split_node_requirement(requirement).0.to_owned()
    } else {
        requirement_package(requirement)
    }
}

/// A package name as the managers compare it: lower case, with each run of `.`, `_` and `-` as `-`.
pub fn normalized_package(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(c, '.' | '_' | '-') {
            if !normalized.ends_with('-') {
                normalized.push('-');
            }
        } else {
            normalized.push(c.to_ascii_lowercase());
        }
    }
    normalized
}

fn tool_problems(steps: &[ToolStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for step in steps {
        if !TOOL_MANAGERS.contains(&step.manager.as_str()) {
            problems.push(format!(
                "the `tools` manager `{}` is not one of: {}",
                step.manager,
                TOOL_MANAGERS.join(", ")
            ));
        }
        problems.extend(group_problem(step));
        if step.packages.is_empty() {
            problems.push("a `tools` step has no packages".to_owned());
        }
        problems.extend(package_problems(step, &mut seen));
    }
    problems
}

/// A group that is not a name, or, for pnpm and npm, a group other than `dev`.
fn group_problem(step: &ToolStep) -> Option<String> {
    let group = step.group.as_deref()?;
    if !is_name(group) {
        return Some(format!("the `tools` group `{group}` is not a name"));
    }
    (is_node_manager(&step.manager) && group != DEFAULT_TOOL_GROUP).then(|| {
        format!(
            "the `tools` group of `{}` is `{DEFAULT_TOOL_GROUP}` (devDependencies), not `{group}`",
            step.manager
        )
    })
}

/// The packages of `step` that are not requirements of its manager, or that `seen` already has.
fn package_problems(step: &ToolStep, seen: &mut BTreeSet<(bool, String)>) -> Vec<String> {
    let node = is_node_manager(&step.manager);
    let mut problems = Vec::new();
    for package in &step.packages {
        let valid = if node {
            is_node_requirement(package)
        } else {
            is_requirement(package)
        };
        if !valid {
            problems.push(format!(
                "the `tools` package `{package}` is not a requirement"
            ));
        } else if !seen.insert((node, tool_package(&step.manager, package))) {
            problems.push(format!("`tools` lists `{package}` twice"));
        }
    }
    problems
}

fn link_problems(links: &[LinkStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for step in links {
        let path = step.path.as_str();
        problems.extend(path_problem("links", path));
        let target = step.target.as_str();
        let relative = !target.trim().is_empty()
            && !target.contains('\\')
            && target.split('/').all(|part| !part.is_empty());
        if !relative {
            problems.push(format!(
                "the `links` step for `{path}` has a target that is not a relative path: \
                 it is empty, starts with `/`, or has an empty part or a `\\`"
            ));
        }
        if !seen.insert(path) {
            problems.push(format!("`links` lists `{path}` twice"));
        }
    }
    problems
}

fn gitignore_problems(steps: &[GitignoreStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut lines = BTreeSet::new();
    let mut unignored = BTreeSet::new();
    for step in steps {
        if step.lines.is_empty() && step.unignore.is_empty() {
            problems.push("a `gitignore` step has no `lines` and no `unignore`".to_owned());
        }
        problems.extend(
            step.lines
                .iter()
                .filter_map(|line| ignored_line_problem(line, &mut lines)),
        );
        problems.extend(
            step.unignore
                .iter()
                .filter_map(|line| unignored_line_problem(line, &mut unignored)),
        );
    }
    problems
}

/// What is wrong with a line of `lines` of a `gitignore` step, given the lines `seen` before it.
fn ignored_line_problem<'a>(line: &'a str, seen: &mut BTreeSet<&'a str>) -> Option<String> {
    if line.trim().is_empty() {
        Some("`gitignore` has an empty line".to_owned())
    } else if line.contains(['\n', '\r']) {
        Some("a `gitignore` line has a line break".to_owned())
    } else if UNIGNORE_WHITELIST.contains(&line.trim()) {
        Some(format!("`gitignore` cannot ignore `{}`", line.trim()))
    } else if !seen.insert(line) {
        Some(format!("`gitignore` lists `{line}` twice"))
    } else {
        None
    }
}

/// What is wrong with a line of `unignore` of a `gitignore` step, given the lines `seen` before it.
fn unignored_line_problem<'a>(line: &'a str, seen: &mut BTreeSet<&'a str>) -> Option<String> {
    if !UNIGNORE_WHITELIST.contains(&line) {
        Some(format!(
            "`unignore` can only name {}, not `{line}`",
            UNIGNORE_WHITELIST
                .map(|name| format!("`{name}`"))
                .join(", ")
        ))
    } else if !seen.insert(line) {
        Some(format!("`unignore` lists `{line}` twice"))
    } else {
        None
    }
}

fn skill_problems(skills: &[SkillStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for step in skills {
        let source = step.source.as_str();
        if source.trim().is_empty() {
            problems.push("a `skills` step has an empty `source`".to_owned());
        }
        if step.names.is_empty() {
            problems.push(format!("the `skills` step for `{source}` has no names"));
        }
        for name in &step.names {
            if name.trim().is_empty() {
                problems.push(format!(
                    "the `skills` step for `{source}` has an empty name"
                ));
            } else if name.chars().any(char::is_whitespace) {
                problems.push(format!("the skill name `{name}` has a space"));
            } else if !seen.insert(name.as_str()) {
                problems.push(format!("`skills` lists `{name}` twice"));
            }
        }
    }
    problems
}

fn file_problems(files: &[FileStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for step in files {
        let path = step.path.as_str();
        problems.extend(path_problem("files", path));
        problems.extend(empty_problem("files", path, "template", &step.template));
        if !seen.insert(path) {
            problems.push(format!("`files` lists `{path}` twice"));
        }
    }
    problems
}

fn section_problems(sections: &[SectionStep]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for step in sections {
        let path = step.path.as_str();
        problems.extend(path_problem("sections", path));
        problems.extend(empty_problem("sections", path, "marker", &step.marker));
        problems.extend(empty_problem("sections", path, "template", &step.template));
        if !seen.insert((path, step.marker.as_str())) {
            problems.push(format!(
                "`sections` lists `{}` of `{path}` twice",
                step.marker
            ));
        }
    }
    problems
}

fn path_problem(kind: &str, path: &str) -> Option<String> {
    let relative = !path.trim().is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    (!relative).then(|| {
        format!(
            "a `{kind}` path `{path}` must be a path relative to the root of the project, \
             without `.` or `..`"
        )
    })
}

fn empty_problem(kind: &str, path: &str, field: &str, value: &str) -> Option<String> {
    value
        .trim()
        .is_empty()
        .then(|| format!("the `{kind}` step for `{path}` has an empty `{field}`"))
}

/// What is wrong with the templates that the steps of one recipe name: each one is under the
/// folder of its kind of step, and `exists` says whether the folder of the recipes has it.
pub(crate) fn template_problems(
    files: &[FileStep],
    sections: &[SectionStep],
    exists: &dyn Fn(&str) -> bool,
) -> Vec<String> {
    let files = files
        .iter()
        .map(|step| ("files", step.path.as_str(), step.template.as_str()));
    let sections = sections
        .iter()
        .map(|step| ("sections", step.path.as_str(), step.template.as_str()));
    files
        .chain(sections)
        .filter(|(_, _, template)| !template.trim().is_empty())
        .filter_map(|(kind, path, template)| {
            let reason = if !template.starts_with(&format!("{kind}/")) {
                format!("is not under `{kind}/`")
            } else if !exists(template) {
                "does not exist".to_owned()
            } else {
                return None;
            };
            Some(format!(
                "the `{kind}` step for `{path}` names the template `{template}`, which {reason}"
            ))
        })
        .collect()
}
