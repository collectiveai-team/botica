//! `botica recipe install`: prepare the repository with the steps of one or more recipes.

use std::process::ExitCode;

use anyhow::{anyhow, Result};
use botica_catalog::{Recipes, ResolvedFile, ResolvedSection, ResolvedSkill};

use crate::recipes_collection::Collection;
use crate::recipes_install_skills as skills;
use crate::recipes_install_steps::{self as steps, Outcome, Template};

#[derive(Default)]
struct Tally {
    added: usize,
    ok: usize,
    pending: usize,
    failed: usize,
}

impl Tally {
    /// Prints the line of a step and counts it.
    fn report(&mut self, kind: &str, id: &str, outcome: &Outcome) {
        match outcome {
            Outcome::Added(_) => self.added += 1,
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
    skills: Vec<ResolvedSkill>,
    warnings: Vec<String>,
}

/// Resolves every name, so that a name that is not a recipe fails before anything is written. A
/// step that two of them share stays once; a skill that a later name takes from another source is
/// taken over by it, with a warning, as the resolution of one recipe does.
fn gather(recipes: &Recipes, names: &[String]) -> Result<Steps> {
    let mut gathered = Steps::default();
    for name in names {
        let resolved = recipes.resolve(name).map_err(anyhow::Error::msg)?;
        gathered.warnings.extend(resolved.warnings);
        for file in resolved.files {
            if !gathered.files.contains(&file) {
                gathered.files.push(file);
            }
        }
        for section in resolved.sections {
            if !gathered.sections.contains(&section) {
                gathered.sections.push(section);
            }
        }
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

/// Adds the files, then the sections, then the skills, of the recipes in `names`, from the root of
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
