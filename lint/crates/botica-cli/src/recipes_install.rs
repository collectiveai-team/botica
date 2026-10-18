//! `botica recipe install`: prepare the repository with the steps of one or more recipes.

use std::process::ExitCode;

use anyhow::{anyhow, Result};
use botica_catalog::{ResolvedFile, ResolvedSection};

use crate::recipes_collection::Collection;
use crate::recipes_install_steps::{self as steps, Outcome, Template};

#[derive(Default)]
struct Tally {
    added: usize,
    ok: usize,
    pending: usize,
}

impl Tally {
    /// Prints the line of a step and counts it.
    fn report(&mut self, kind: &str, id: &str, outcome: &Outcome) {
        match outcome {
            Outcome::Added(_) => self.added += 1,
            Outcome::Present => self.ok += 1,
            Outcome::Pending(_) => self.pending += 1,
        }
        println!("{kind} {id}: {}", outcome.text());
    }
}

/// Adds the files, then the sections, of the recipes in `names`, from the root of the project. A
/// step that two of them share is done once. Nothing is written when a name is not a recipe.
pub fn run(collection: &Collection, names: &[String]) -> Result<ExitCode> {
    let recipes = collection.loaded()?;
    let mut files: Vec<ResolvedFile> = Vec::new();
    let mut sections: Vec<ResolvedSection> = Vec::new();
    let mut warnings = Vec::new();
    for name in names {
        let resolved = recipes.resolve(name).map_err(anyhow::Error::msg)?;
        warnings.extend(resolved.warnings);
        for file in resolved.files {
            if !files.contains(&file) {
                files.push(file);
            }
        }
        for section in resolved.sections {
            if !sections.contains(&section) {
                sections.push(section);
            }
        }
    }
    for warning in &warnings {
        eprintln!("warning: {warning}");
    }
    let root = collection.project.loaded.root.as_path();
    let text = |recipe: &str, path: &str| {
        recipes
            .template(recipe, path)
            .ok_or_else(|| anyhow!("the template `{path}` of `{recipe}` is missing"))
    };
    let mut tally = Tally::default();
    for item in &files {
        let template = Template {
            text: text(&item.from, &item.template)?,
            path: &item.template,
            recipe: &item.from,
        };
        let outcome = steps::add_file(root, &item.path, &template)?;
        tally.report("file", &item.path, &outcome);
    }
    for item in &sections {
        let template = text(&item.from, &item.template)?;
        let outcome = steps::add_section(root, &item.path, &item.marker, template)?;
        let id = format!("{} `{}`", item.path, item.marker);
        tally.report("section", &id, &outcome);
    }
    println!(
        "{}: {} added, {} ok, {} pending",
        names.join(", "),
        tally.added,
        tally.ok,
        tally.pending
    );
    Ok(ExitCode::SUCCESS)
}
