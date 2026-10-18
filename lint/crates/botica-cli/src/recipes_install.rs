//! `botica recipe install`: prepare the repository with the steps of one or more recipes.

use std::process::ExitCode;

use anyhow::{anyhow, Result};
use botica_catalog::{
    RecipeItem, Recipes, ResolvedFile, ResolvedLink, ResolvedSection, ResolvedSkill, ResolvedTool,
};

use crate::recipes_collection::Collection;
use crate::recipes_install_gitignore as gitignore;
use crate::recipes_install_skills as skills;
use crate::recipes_install_steps::{self as steps, Outcome, Template};
use crate::recipes_install_tools as tools;

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

/// The steps of the recipes that were named, each one once.
#[derive(Default)]
struct Steps {
    files: Vec<ResolvedFile>,
    sections: Vec<ResolvedSection>,
    links: Vec<ResolvedLink>,
    gitignore: Vec<RecipeItem>,
    unignore: Vec<RecipeItem>,
    tools: Vec<ResolvedTool>,
    skills: Vec<ResolvedSkill>,
    warnings: Vec<String>,
}

/// Adds the items that `items` does not have yet, by their line.
fn extend_once(items: &mut Vec<RecipeItem>, more: Vec<RecipeItem>) {
    for item in more {
        if !items.iter().any(|known| known.id == item.id) {
            items.push(item);
        }
    }
}

/// Resolves every name, so that a name that is not a recipe fails before anything is written. A
/// step that two of them list the same stays once, whatever recipe lists it; a skill that a later
/// name takes from another source is taken over by it, with a warning, as the resolution of one
/// recipe does.
fn gather(recipes: &Recipes, names: &[String]) -> Result<Steps> {
    let mut gathered = Steps::default();
    for name in names {
        let resolved = recipes.resolve(name).map_err(anyhow::Error::msg)?;
        gathered.warnings.extend(resolved.warnings);
        for file in resolved.files {
            let listed =
                |known: &ResolvedFile| known.path == file.path && known.template == file.template;
            if !gathered.files.iter().any(listed) {
                gathered.files.push(file);
            }
        }
        for section in resolved.sections {
            let listed = |known: &ResolvedSection| {
                known.path == section.path
                    && known.marker == section.marker
                    && known.template == section.template
            };
            if !gathered.sections.iter().any(listed) {
                gathered.sections.push(section);
            }
        }
        for link in resolved.links {
            let listed =
                |known: &ResolvedLink| known.path == link.path && known.target == link.target;
            if !gathered.links.iter().any(listed) {
                gathered.links.push(link);
            }
        }
        for tool in resolved.tools {
            let listed = |known: &ResolvedTool| {
                known.manager == tool.manager
                    && known.group == tool.group
                    && known.package == tool.package
            };
            if !gathered.tools.iter().any(listed) {
                gathered.tools.push(tool);
            }
        }
        extend_once(&mut gathered.gitignore, resolved.gitignore);
        extend_once(&mut gathered.unignore, resolved.unignore);
        for skill in resolved.skills {
            match gathered
                .skills
                .iter_mut()
                .find(|known| known.name == skill.name)
            {
                None => gathered.skills.push(skill),
                Some(known) if known.source == skill.source => {}
                Some(known) => {
                    gathered.warnings.push(format!(
                        "the skill `{}` comes from `{}` ({}), which replaces `{}` ({})",
                        skill.name, skill.source, skill.from, known.source, known.from
                    ));
                    *known = skill;
                }
            }
        }
    }
    Ok(gathered)
}

/// Adds the files, then the sections, then the links, then the lines of `.gitignore`, then the tools, then the skills, of the recipes in `names`, from the root of
/// the project. The command exits with 1 when a step is pending or a skill is not installed at the
/// end.
pub fn run(collection: &Collection, names: &[String]) -> Result<ExitCode> {
    let recipes = collection.loaded()?;
    let gathered = gather(recipes, names)?;
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
    for (item, outcome) in gathered
        .skills
        .iter()
        .zip(skills::install(root, &gathered.skills))
    {
        tally.report("skill", &item.name, &outcome);
    }
    println!("{}", tally.summary(names));
    Ok(if tally.failed == 0 && tally.pending == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
