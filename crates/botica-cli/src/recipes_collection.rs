use anyhow::{bail, Result};
use botica_catalog::{read_recipes, CatalogError, Recipes, RECIPES_DIR};

use crate::project::{self, Project, ProjectArgs};

/// What `botica recipe` reads: the project, with the rules of every catalog, and the recipes of
/// the binary together with those of its `recipes/`.
pub struct Collection {
    pub project: Project,
    pub recipes: Result<Recipes, CatalogError>,
}

pub fn open(args: &ProjectArgs) -> Result<Collection> {
    let project = project::load(args)?;
    let files = read_recipes(&project.loaded.root.join(RECIPES_DIR))?;
    let builtin = Recipes::embedded()?;
    let replace = &project.loaded.config.recipes.replace;
    Ok(Collection {
        recipes: Recipes::over(&builtin, &files, replace),
        project,
    })
}

impl Collection {
    /// The recipes, when the built-in ones and `recipes/` load together.
    pub fn loaded(&self) -> Result<&Recipes> {
        match &self.recipes {
            Ok(recipes) => Ok(recipes),
            Err(error) => bail!("{RECIPES_DIR}/ does not load:\n{error}"),
        }
    }
}
