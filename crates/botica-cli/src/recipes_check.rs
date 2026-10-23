//! `botica recipe check`: how the project differs from a recipe.

use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::{Kind, ResolvedRecipe};
use botica_core::walk;

use crate::project::ProjectArgs;
use crate::recipes_check_fragments as fragments;
use crate::recipes_check_steps as steps;
use crate::recipes_collection::Collection;
use crate::recipes_pyproject::Pyproject;
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

/// One line for each rule, judgment rule, fragment of an external rule, export, file, section,
/// link, line of `.gitignore`, tool and skill of the recipes in `names`, in that order, then a
/// summary. The command fails when a rule is not active, a fragment or an export is not current,
/// or a step is not in the project. The paths of the steps are from the root of the project.
pub fn run(collection: &Collection, names: &[String], project: &ProjectArgs) -> Result<ExitCode> {
    let resolved = collection
        .loaded()?
        .resolve_many(names)
        .map_err(anyhow::Error::msg)?;
    for warning in &resolved.warnings {
        eprintln!("warning: {warning}");
    }
    let root = collection.project.loaded.root.as_path();
    let mut report = Report::default();
    report.check_rules(collection, &resolved, project, root);
    report.check_steps(&resolved, root);
    Ok(report.print(&resolved.id))
}

impl Report {
    /// Checks the rules, the judgment rules, the fragments of the external rules and the exports.
    fn check_rules(
        &mut self,
        collection: &Collection,
        resolved: &ResolvedRecipe,
        project: &ProjectArgs,
        root: &Path,
    ) {
        for item in &resolved.rules {
            self.add("rule", &item.id, rule_problem(collection, &item.id, false));
        }
        for item in &resolved.judgment {
            self.add(
                "judgment rule",
                &item.id,
                rule_problem(collection, &item.id, true),
            );
        }
        let files = walk::files(root, None);
        for item in &resolved.rules {
            for (id, problem) in fragments::of_rule(collection, &item.id, root, &files) {
                self.add("fragment", &id, problem);
            }
        }
        for item in &resolved.exports {
            let problem = rules_export::check_export(&item.id, project)
                .err()
                .map(|error| error.to_string());
            self.add("export", &item.id, problem);
        }
    }

    /// Checks the files, sections, links, lines of `.gitignore`, tools and skills.
    fn check_steps(&mut self, resolved: &ResolvedRecipe, root: &Path) {
        for item in &resolved.files {
            self.add("file", &item.path, steps::file(root, &item.path));
        }
        for item in &resolved.sections {
            let id = format!("{} `{}`", item.path, item.marker);
            self.add(
                "section",
                &id,
                steps::section(root, &item.path, &item.marker),
            );
        }
        for item in &resolved.links {
            self.add(
                "link",
                &item.path,
                steps::link(root, &item.path, &item.target),
            );
        }
        for item in &resolved.gitignore {
            self.add("gitignore", &item.id, steps::gitignore_line(root, &item.id));
        }
        for item in &resolved.unignore {
            self.add("unignore", &item.id, steps::unignored(root, &item.id));
        }
        if !resolved.tools.is_empty() {
            let pyproject = Pyproject::read(root);
            for item in &resolved.tools {
                self.add("tool", &item.package, pyproject.problem(&item.package));
            }
        }
        if !resolved.skills.is_empty() {
            let lock = steps::Lock::read(root);
            for item in &resolved.skills {
                self.add("skill", &item.name, steps::skill(root, &lock, &item.name));
            }
        }
    }

    /// Prints the lines and the summary of the recipe `id`: the command fails when a line differs.
    fn print(&self, id: &str) -> ExitCode {
        for line in &self.lines {
            println!("{line}");
        }
        let differ = self.differ;
        let verb = if differ == 1 { "differs" } else { "differ" };
        println!("{id}: {} checked, {differ} {verb}", self.lines.len());
        if differ == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }
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
