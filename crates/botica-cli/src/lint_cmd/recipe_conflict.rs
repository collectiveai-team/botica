//! `recipe-conflict` over a project: the recipes it installs and those of its `recipes/`, with
//! what they extend, resolved together as `botica recipe install` resolves the names it gets.

use std::path::Path;

use botica_catalog::{read_recipes, Origin, Recipes, ToolConflict, RECIPES_DIR};
use botica_config::Resolution;
use botica_core::recipe_conflict::{check, unresolved, RecipeFile, RECIPE_CONFLICT};
use botica_core::{Diagnostic, Severity};

use super::project_files;
use crate::project::Project;

/// What `recipe-conflict` reads of the configuration.
#[derive(Debug, Clone, Default)]
pub struct ProjectRecipes {
    /// `installed` of `[recipes]`.
    pub installed: Vec<String>,
    /// `replace` of `[recipes]`.
    pub replace: Vec<String>,
    /// The file of the configuration from the root of the project, `botica.toml` when there is
    /// none: a finding that no recipe of `recipes/` holds is about it.
    pub config_file: String,
}

impl ProjectRecipes {
    pub fn of(project: &Project) -> Self {
        let loaded = &project.loaded;
        let config_file = loaded
            .source
            .as_deref()
            .and_then(|source| source.strip_prefix(&loaded.root).ok())
            .map_or_else(
                || "botica.toml".to_owned(),
                |path| path.to_string_lossy().replace('\\', "/"),
            );
        Self {
            installed: loaded.config.recipes.installed.clone(),
            replace: loaded.config.recipes.replace.clone(),
            config_file,
        }
    }
}

/// What `recipe-conflict` finds, once for the whole project at `root`, shown from the directory
/// `prefix` inside it.
pub fn findings(
    resolution: &Resolution,
    recipes: &ProjectRecipes,
    root: &Path,
    prefix: Option<&str>,
) -> Vec<Diagnostic> {
    let Some(entry) = resolution
        .entry(RECIPE_CONFLICT)
        .filter(|entry| entry.active)
    else {
        return Vec::new();
    };
    let severity = Severity::from(entry.severity);
    let files = match read_recipes(&root.join(RECIPES_DIR)) {
        Ok(files) => files,
        Err(error) => return vec![unresolved(RECIPES_DIR, &error.to_string(), severity)],
    };
    let loaded =
        Recipes::embedded().and_then(|builtin| Recipes::over(&builtin, &files, &recipes.replace));
    let mut found = match loaded {
        Err(error) => error
            .issues
            .iter()
            .map(|issue| {
                let path = if issue.path.starts_with('[') {
                    recipes.config_file.clone()
                } else {
                    format!("{RECIPES_DIR}/{}", issue.path)
                };
                unresolved(&path, &issue.message, severity)
            })
            .collect(),
        Ok(loaded) => match conflicts(&loaded, &recipes.installed) {
            Err(problem) => vec![unresolved(&recipes.config_file, &problem, severity)],
            Ok(conflicts) => {
                let texts: Vec<(String, String, &str)> = files
                    .iter()
                    .filter_map(|file| {
                        let id = file.path.strip_suffix(".toml")?;
                        let path = format!("{RECIPES_DIR}/{}", file.path);
                        (!id.contains('/')).then(|| (id.to_owned(), path, file.text.as_str()))
                    })
                    .collect();
                let files: Vec<RecipeFile<'_>> = texts
                    .iter()
                    .map(|(id, path, text)| RecipeFile { id, path, text })
                    .collect();
                check(&conflicts, &files, &recipes.config_file, severity)
            }
        },
    };
    project_files::seen_from(prefix, &mut found);
    found
}

/// The conflicts of the recipes `first` (the installed ones, or the names that `botica recipe
/// check` gets with them) together with the recipes of `recipes/`.
pub fn conflicts(loaded: &Recipes, first: &[String]) -> Result<Vec<ToolConflict>, String> {
    let mut ids: Vec<&str> = Vec::new();
    let project = loaded
        .recipes()
        .iter()
        .filter(|recipe| recipe.origin == Origin::Project)
        .map(|recipe| recipe.id.as_str());
    for id in first.iter().map(String::as_str).chain(project) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    loaded.tool_conflicts(&ids)
}
