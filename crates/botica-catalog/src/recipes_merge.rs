use crate::issue::{issue, CatalogError};
use crate::load::SourceFile;
use crate::recipes::{recipe_path, Recipe, Recipes};

/// The path of the problems with `replace`: they are in the configuration, not in a file of
/// `recipes/`.
const REPLACE_KEY: &str = "[recipes] replace";

impl Recipes {
    /// The recipes of a project together with the built-in ones, in the order of their ids. A
    /// recipe of the project with the id of a built-in recipe is an error unless `replace` names
    /// the id; then it takes the place of the built-in recipe. A recipe of the project may extend
    /// a built-in one.
    pub fn over(
        builtin: &Recipes,
        project: &[SourceFile],
        replace: &[String],
    ) -> Result<Self, CatalogError> {
        let project = Self::from_files(project)?;
        let mut issues = Vec::new();
        for id in replace {
            match (builtin.recipe(id), project.recipe(id)) {
                (None, _) => issues.push(issue(
                    REPLACE_KEY,
                    format!("`{id}` is not a built-in recipe"),
                )),
                (Some(_), None) => issues.push(issue(
                    REPLACE_KEY,
                    format!(
                        "`{id}` is a built-in recipe, and `recipes/` has no recipe with that id"
                    ),
                )),
                (Some(_), Some(_)) => {}
            }
        }
        for recipe in &project.recipes {
            if builtin.recipe(&recipe.id).is_some() && !replace.contains(&recipe.id) {
                issues.push(issue(
                    recipe_path(&recipe.id),
                    format!(
                        "the id `{}` is the one of a built-in recipe; to replace that recipe, \
                         list it in [recipes] replace in the configuration",
                        recipe.id
                    ),
                ));
            }
        }
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }
        let mut recipes: Vec<Recipe> = builtin
            .recipes
            .iter()
            .filter(|recipe| !replace.contains(&recipe.id))
            .cloned()
            .chain(project.recipes)
            .collect();
        recipes.sort_by(|a, b| a.id.cmp(&b.id));
        let templates = builtin
            .templates
            .iter()
            .cloned()
            .chain(project.templates)
            .collect();
        let all = Self { recipes, templates };
        let issues = all.extends_problems();
        if issues.is_empty() {
            Ok(all)
        } else {
            Err(CatalogError { issues })
        }
    }
}
