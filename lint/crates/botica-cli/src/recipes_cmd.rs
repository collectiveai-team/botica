use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::{Origin, Recipe, RecipeItem, ResolvedRecipe};
use clap::{Args, Subcommand};

use crate::project::ProjectArgs;
use crate::recipes_check;
use crate::recipes_collection::{self, Collection};

#[derive(Args)]
pub struct RecipesArgs {
    #[command(subcommand)]
    command: RecipesCommand,
}

#[derive(Subcommand)]
enum RecipesCommand {
    /// List the recipes of the binary and of `recipes/`, with what each one extends and lists.
    List(ListArgs),
    /// Print a recipe with the rules and the exports of the recipes it extends.
    Show(NameArgs),
    /// Report how the project differs from a recipe: the rules that are not active and the
    /// exports that are not current.
    Check(NameArgs),
}

#[derive(Args)]
struct ListArgs {
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Args)]
struct NameArgs {
    /// The id of a recipe (`botica recipe list` lists them).
    name: String,
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: &RecipesArgs) -> Result<ExitCode> {
    match &args.command {
        RecipesCommand::List(args) => list(&recipes_collection::open(&args.project)?),
        RecipesCommand::Show(args) => show(&recipes_collection::open(&args.project)?, &args.name),
        RecipesCommand::Check(args) => recipes_check::run(
            &recipes_collection::open(&args.project)?,
            &args.name,
            &args.project,
        ),
    }
}

fn list(collection: &Collection) -> Result<ExitCode> {
    let blocks: Vec<String> = collection
        .loaded()?
        .recipes()
        .iter()
        .map(list_block)
        .collect();
    print!("{}", blocks.concat());
    Ok(ExitCode::SUCCESS)
}

fn list_block(recipe: &Recipe) -> String {
    let mut block = match &recipe.description {
        Some(description) => format!("{}: {description}\n", recipe.id),
        None => format!("{}\n", recipe.id),
    };
    if !recipe.extends.is_empty() {
        let _ = writeln!(block, "  extends: {}", recipe.extends.join(", "));
    }
    let _ = writeln!(
        block,
        "  rules: {}, judgment: {}, exports: {}",
        recipe.rules.len(),
        recipe.judgment.len(),
        recipe.exports.len()
    );
    let from = match recipe.origin {
        Origin::BuiltIn => "built-in",
        Origin::Project => "project",
    };
    let _ = writeln!(block, "  from: {from}");
    block
}

fn show(collection: &Collection, name: &str) -> Result<ExitCode> {
    let recipes = collection.loaded()?;
    let resolved = recipes.resolve(name).map_err(anyhow::Error::msg)?;
    let recipe = recipes.recipe(name);
    print!("{}", show_text(collection, recipe, &resolved));
    Ok(ExitCode::SUCCESS)
}

fn show_text(
    collection: &Collection,
    recipe: Option<&Recipe>,
    resolved: &ResolvedRecipe,
) -> String {
    let mut text = match recipe.and_then(|recipe| recipe.description.as_deref()) {
        Some(description) => format!("# {}: {description}\n\n", resolved.id),
        None => format!("# {}\n\n", resolved.id),
    };
    if let Some(recipe) = recipe.filter(|recipe| !recipe.extends.is_empty()) {
        let _ = write!(text, "Extends: {}\n\n", recipe.extends.join(", "));
    }
    let sections = [
        ("Rules", &resolved.rules, true),
        ("Judgment rules", &resolved.judgment, true),
        ("Exports", &resolved.exports, false),
    ];
    for (title, items, rules) in sections {
        let _ = write!(text, "## {title}\n\n");
        if items.is_empty() {
            text.push_str("(none)\n");
        }
        for item in items {
            let _ = writeln!(text, "- {}", describe(collection, item, rules));
        }
        text.push('\n');
    }
    text.truncate(text.trim_end().len());
    text.push('\n');
    text
}

/// An entry as `show` prints it: a rule with its code and its summary, or an export.
fn describe(collection: &Collection, item: &RecipeItem, rule: bool) -> String {
    if !rule {
        return format!("{} (from {})", item.id, item.from);
    }
    match collection.project.catalog.resolve(&item.id) {
        Some(found) => format!(
            "{} ({}, from {}): {}",
            item.id, found.code, item.from, found.summary
        ),
        None => format!("{} (not in the catalog, from {})", item.id, item.from),
    }
}
