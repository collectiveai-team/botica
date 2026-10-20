//! `botica recipe install`: prepare the repository with the steps of one or more recipes.

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use botica_catalog::Status;

use crate::project::ProjectArgs;
use crate::recipes_collection::Collection;
use crate::recipes_install_gitignore as gitignore;
use crate::recipes_install_selection as selection;
use crate::recipes_install_skills as skills;
use crate::recipes_install_steps::{self as steps, Outcome, Template};
use crate::recipes_install_tools as tools;
use crate::{rubric, rules_export};

#[derive(Default)]
struct Tally {
    added: usize,
    edited: usize,
    ok: usize,
    pending: usize,
    failed: usize,
}

impl Tally {
    /// Prints the line of a step and counts it.
    fn report(&mut self, kind: &str, id: &str, outcome: &Outcome) {
        match outcome {
            Outcome::Added(_) => self.added += 1,
            Outcome::Edited(_) => self.edited += 1,
            Outcome::Present => self.ok += 1,
            Outcome::Pending(_) => self.pending += 1,
            Outcome::Failed(_) => self.failed += 1,
        }
        println!("{kind} {id}: {}", outcome.text());
    }

    /// The last line, with the failures only when there are any.
    fn summary(&self, names: &[String]) -> String {
        let mut line = format!(
            "{}: {} added, {} ok, {} pending",
            names.join(", "),
            self.added,
            self.ok,
            self.pending
        );
        if self.edited > 0 {
            line.push_str(&format!(", {} edited", self.edited));
        }
        if self.failed > 0 {
            line.push_str(&format!(", {} failed", self.failed));
        }
        line
    }
}

/// The file that the export `name` writes, from the root of the project.
fn export_file(name: &str) -> &'static str {
    match name {
        "abide" => rubric::DEFAULT_OUTPUT,
        _ => rubric::DEFAULT_AGENTS_MD,
    }
}

/// Runs the export `name` that a recipe asks for, after the sections that give `AGENTS.md` its
/// block, and says whether its file changed.
fn export(root: &Path, name: &str, project: &ProjectArgs) -> Outcome {
    let file = root.join(export_file(name));
    let before = fs::read_to_string(&file).ok();
    if let Err(error) = rules_export::write_export(name, project) {
        return Outcome::Pending(format!("{error:#}"));
    }
    match (before, fs::read_to_string(&file).ok()) {
        (None, Some(_)) => Outcome::Added(None),
        (Some(old), Some(new)) if old != new => Outcome::Edited("wrote the judgment rules"),
        _ => Outcome::Present,
    }
}

/// Writes the selection of the rules, then adds the files, then the sections, then the links, then the lines of `.gitignore`, then the tools, then the skills, of the recipes in `names`, from the root of
/// the project. The command exits with 1 when a step is pending or a skill is not installed at the
/// end.
pub fn run(collection: &Collection, names: &[String], project: &ProjectArgs) -> Result<ExitCode> {
    let recipes = collection.loaded()?;
    let gathered = recipes.resolve_many(names).map_err(anyhow::Error::msg)?;
    for warning in &gathered.warnings {
        eprintln!("warning: {warning}");
    }
    let root = collection.project.loaded.root.as_path();
    let text = |recipe: &str, path: &str| {
        recipes
            .template(recipe, path)
            .ok_or_else(|| anyhow!("the template `{path}` of `{recipe}` is missing"))
    };
    let mut tally = Tally::default();
    let rules: Vec<&str> = gathered
        .rules
        .iter()
        .chain(&gathered.judgment)
        .map(|item| item.id.as_str())
        .collect();
    let preview = rules.iter().any(|rule| {
        collection
            .project
            .catalog
            .resolve(rule)
            .is_some_and(|rule| rule.status == Status::Preview)
    });
    if !rules.is_empty() {
        let outcome = selection::add(root, names, &rules, preview)?;
        tally.report("selection", selection::BOTICA_TOML, &outcome);
    }
    for item in &gathered.files {
        let template = Template {
            text: text(&item.from, &item.template)?,
            path: &item.template,
            recipe: &item.from,
        };
        let outcome = steps::add_file(root, &item.path, &template)?;
        tally.report("file", &item.path, &outcome);
    }
    for item in &gathered.sections {
        let template = text(&item.from, &item.template)?;
        let outcome = steps::add_section(root, &item.path, &item.marker, template)?;
        let id = format!("{} `{}`", item.path, item.marker);
        tally.report("section", &id, &outcome);
    }
    for item in &gathered.links {
        let outcome = steps::add_link(root, &item.path, &item.target)?;
        tally.report("link", &item.path, &outcome);
    }
    let lines: Vec<&str> = gathered
        .gitignore
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    for (item, outcome) in gathered
        .gitignore
        .iter()
        .zip(gitignore::add_lines(root, &lines)?)
    {
        tally.report("gitignore", &item.id, &outcome);
    }
    for item in &gathered.unignore {
        let outcome = gitignore::unignore(root, &item.id)?;
        tally.report("unignore", &item.id, &outcome);
    }
    for (item, outcome) in gathered
        .tools
        .iter()
        .zip(tools::install(root, &gathered.tools))
    {
        tally.report("tool", &item.package, &outcome);
    }
    let agents: Vec<&str> = gathered
        .agents
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    for (item, outcome) in
        gathered
            .skills
            .iter()
            .zip(skills::install(root, &gathered.skills, &agents))
    {
        tally.report("skill", &item.name, &outcome);
    }
    for item in &gathered.exports {
        let outcome = export(root, &item.id, project);
        tally.report("export", &item.id, &outcome);
    }
    println!("{}", tally.summary(names));
    Ok(if tally.failed == 0 && tally.pending == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
