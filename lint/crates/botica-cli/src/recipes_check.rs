//! `botica recipe check`: how the project differs from a recipe.

use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::Kind;

use crate::project::ProjectArgs;
use crate::recipes_check_steps as steps;
use crate::recipes_collection::Collection;
use crate::rules_export;

/// The lines of the report and how many of them differ.
#[derive(Default)]
struct Report {
    lines: Vec<String>,
    differ: usize,
}

impl Report {
    fn add(&mut self, kind: &str, id: &str, problem: Option<String>) {
        self.lines.push(match problem {
            None => format!("{kind} {id}: ok"),
            Some(problem) => {
                self.differ += 1;
                format!("{kind} {id}: differs: {problem}")
            }
        });
    }
}

/// One line for each rule, export, file, section, line of `.gitignore` and skill of the recipe, in
/// that order, then a
/// summary. The command fails when a rule is not active, an export is not current, or a step is
/// not in the project. The paths of the steps are from the root of the project.
pub fn run(collection: &Collection, name: &str, project: &ProjectArgs) -> Result<ExitCode> {
    let resolved = collection
        .loaded()?
        .resolve(name)
        .map_err(anyhow::Error::msg)?;
    let root = collection.project.loaded.root.as_path();
    let mut report = Report::default();
    for item in &resolved.rules {
        report.add("rule", &item.id, rule_problem(collection, &item.id, false));
    }
    for item in &resolved.judgment {
        report.add(
            "judgment rule",
            &item.id,
            rule_problem(collection, &item.id, true),
        );
    }
    for item in &resolved.exports {
        let problem = rules_export::check_export(&item.id, project)
            .err()
            .map(|error| error.to_string());
        report.add("export", &item.id, problem);
    }
    for item in &resolved.files {
        report.add("file", &item.path, steps::file(root, &item.path));
    }
    for item in &resolved.sections {
        let id = format!("{} `{}`", item.path, item.marker);
        report.add(
            "section",
            &id,
            steps::section(root, &item.path, &item.marker),
        );
    }
    for item in &resolved.gitignore {
        report.add("gitignore", &item.id, steps::gitignore_line(root, &item.id));
    }
    for item in &resolved.unignore {
        report.add("unignore", &item.id, steps::unignored(root, &item.id));
    }
    if !resolved.skills.is_empty() {
        let lock = steps::Lock::read(root);
        for item in &resolved.skills {
            report.add("skill", &item.name, steps::skill(root, &lock, &item.name));
        }
    }
    for line in &report.lines {
        println!("{line}");
    }
    let differ = report.differ;
    let verb = if differ == 1 { "differs" } else { "differ" };
    println!("{name}: {} checked, {differ} {verb}", report.lines.len());
    Ok(if differ == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// Why the project does not run the rule `key` as the recipe lists it, if it does not.
fn rule_problem(collection: &Collection, key: &str, judgment: bool) -> Option<String> {
    let Some(entry) = collection.project.resolution.entry(key) else {
        return Some("not in the catalog".to_owned());
    };
    let is_judgment = entry
        .rule
        .implementations
        .iter()
        .any(|implementation| implementation.kind == Kind::Judgment);
    if judgment && !is_judgment {
        return Some("is not a judgment rule".to_owned());
    }
    if !judgment && is_judgment {
        return Some("is a judgment rule: list it under `judgment`".to_owned());
    }
    (!entry.active).then(|| entry.reason.to_string())
}
