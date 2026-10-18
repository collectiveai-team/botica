use std::collections::BTreeSet;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;
use crate::references::Origin;
use crate::validate::is_kebab_case;

/// The folder of the repository that holds the recipes of a project, beside `references/`.
pub const RECIPES_DIR: &str = "recipes";
const RECIPE_EXTENSION: &str = ".toml";

/// The exports that a recipe can ask for: `botica rules export --format <name>`.
pub const EXPORTS: [&str; 2] = ["agents-md", "abide"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeFile {
    description: Option<String>,
    #[serde(default)]
    extends: Vec<String>,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    judgment: Vec<String>,
    #[serde(default)]
    exports: Vec<String>,
}

/// A file `<id>.toml` of the catalog: the rules and the exports that a repository of some kind
/// has, and the recipes whose lists it adds to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    /// The name of its file, without the extension.
    pub id: String,
    pub origin: Origin,
    pub description: Option<String>,
    pub extends: Vec<String>,
    /// Rules that are not judgment rules, by slug, code or alias.
    pub rules: Vec<String>,
    /// Judgment rules, by slug, code or alias.
    pub judgment: Vec<String>,
    /// Names of `EXPORTS`.
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Recipes {
    pub(crate) recipes: Vec<Recipe>,
}

impl Recipes {
    /// Loads the recipes of a project from the files of `recipes/`, with paths from that folder:
    /// `<recipe>.toml`. What a recipe extends is not checked: it may be a built-in recipe.
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        Self::load(files, Origin::Project)
    }

    /// Loads the recipes that a binary carries. They extend one another and nothing else.
    pub fn from_builtin_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        let recipes = Self::load(files, Origin::BuiltIn)?;
        let issues = recipes.extends_problems();
        if issues.is_empty() {
            Ok(recipes)
        } else {
            Err(CatalogError { issues })
        }
    }

    fn load(files: &[SourceFile], origin: Origin) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let mut recipes: Vec<Recipe> = files
            .iter()
            .filter_map(|file| load_recipe(file, origin, &mut issues))
            .collect();
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }
        recipes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self { recipes })
    }

    pub fn recipes(&self) -> &[Recipe] {
        &self.recipes
    }

    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.id == id)
    }
}

pub(crate) fn recipe_path(id: &str) -> String {
    format!("{id}{RECIPE_EXTENSION}")
}

fn load_recipe(
    file: &SourceFile,
    origin: Origin,
    issues: &mut Vec<CatalogIssue>,
) -> Option<Recipe> {
    let path = file.path.as_str();
    let id = match path.strip_suffix(RECIPE_EXTENSION) {
        Some(id) if !id.contains('/') => id,
        _ => {
            issues.push(issue(path, "unexpected file: expected <recipe>.toml"));
            return None;
        }
    };
    if !is_kebab_case(id) {
        issues.push(issue(
            path,
            format!("the file name `{id}` must be a kebab-case recipe id"),
        ));
        return None;
    }
    let parsed: RecipeFile = match toml::from_str(&file.text) {
        Ok(parsed) => parsed,
        Err(error) => {
            issues.push(issue(path, error.message().to_owned()));
            return None;
        }
    };
    let problems = list_problems(&parsed);
    issues.extend(problems.into_iter().map(|message| issue(path, message)));
    Some(Recipe {
        id: id.to_owned(),
        origin,
        description: parsed.description,
        extends: parsed.extends,
        rules: parsed.rules,
        judgment: parsed.judgment,
        exports: parsed.exports,
    })
}

fn list_problems(parsed: &RecipeFile) -> Vec<String> {
    let lists = [
        ("extends", &parsed.extends),
        ("rules", &parsed.rules),
        ("judgment", &parsed.judgment),
        ("exports", &parsed.exports),
    ];
    let mut problems = Vec::new();
    for (key, entries) in lists {
        let mut seen = BTreeSet::new();
        for entry in entries {
            if entry.trim().is_empty() {
                problems.push(format!("`{key}` has an empty entry"));
            } else if !seen.insert(entry.as_str()) {
                problems.push(format!("`{key}` lists `{entry}` twice"));
            }
        }
    }
    for export in &parsed.exports {
        if !EXPORTS.contains(&export.as_str()) {
            problems.push(format!(
                "unknown export `{export}` (expected one of {})",
                EXPORTS.join(", ")
            ));
        }
    }
    problems
}
