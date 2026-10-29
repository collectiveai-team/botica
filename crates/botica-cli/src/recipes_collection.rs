use std::collections::BTreeMap;

use anyhow::{bail, Result};
use botica_catalog::{read_recipes, CatalogError, Recipes, ResolvedRecipe, RECIPES_DIR};

use crate::project::{self, Project, ProjectArgs};

/// What `botica recipe` reads: the project, with the rules of every catalog, and the recipes of
/// the binary together with those of its `recipes/`.
pub struct Collection {
    pub project: Project,
    pub recipes: Result<Recipes, CatalogError>,
}

pub fn open(args: &ProjectArgs) -> Result<Collection> {
    let project = project::load_describing(args)?;
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

    /// The recipes `names` as one, with the values of their parameters: `set` (`--set`) before
    /// `[recipes.params]` of the configuration before the default.
    pub fn resolve<S: AsRef<str>>(
        &self,
        names: &[S],
        set: &BTreeMap<String, String>,
    ) -> Result<ResolvedRecipe> {
        let recipes = self.loaded()?;
        let mut resolved = recipes.resolve_many(names).map_err(anyhow::Error::msg)?;
        let config: BTreeMap<String, String> = self
            .project
            .loaded
            .config
            .recipes
            .params
            .iter()
            .map(|(name, value)| (name.clone(), value.text()))
            .collect();
        recipes
            .set_params(&mut resolved, &config, set)
            .map_err(anyhow::Error::msg)?;
        Ok(resolved)
    }
}
