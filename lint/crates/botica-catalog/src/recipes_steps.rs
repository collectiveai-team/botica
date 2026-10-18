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

/// The one line that `unignore` can name: the manifest of skills (ADR 0001, CES-107).
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
) -> Vec<String> {
    let mut problems = skill_problems(skills);
    problems.extend(file_problems(files));
    problems.extend(section_problems(sections));
    problems.extend(link_problems(links));
    problems.extend(gitignore_problems(gitignore));
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
        for line in &step.lines {
            if line.trim().is_empty() {
                problems.push("`gitignore` has an empty line".to_owned());
            } else if line.contains(['\n', '\r']) {
                problems.push("a `gitignore` line has a line break".to_owned());
            } else if UNIGNORE_WHITELIST.contains(&line.trim()) {
                problems.push(format!("`gitignore` cannot ignore `{}`", line.trim()));
            } else if !lines.insert(line.as_str()) {
                problems.push(format!("`gitignore` lists `{line}` twice"));
            }
        }
        for line in &step.unignore {
            if !UNIGNORE_WHITELIST.contains(&line.as_str()) {
                problems.push(format!(
                    "`unignore` can only name {}, not `{line}`",
                    UNIGNORE_WHITELIST
                        .map(|name| format!("`{name}`"))
                        .join(", ")
                ));
            } else if !unignored.insert(line.as_str()) {
                problems.push(format!("`unignore` lists `{line}` twice"));
            }
        }
    }
    problems
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
