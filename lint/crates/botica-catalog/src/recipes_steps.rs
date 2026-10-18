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

/// What is wrong with the steps of one recipe, one message for each problem.
pub(crate) fn step_problems(
    skills: &[SkillStep],
    files: &[FileStep],
    sections: &[SectionStep],
) -> Vec<String> {
    let mut problems = skill_problems(skills);
    problems.extend(file_problems(files));
    problems.extend(section_problems(sections));
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
