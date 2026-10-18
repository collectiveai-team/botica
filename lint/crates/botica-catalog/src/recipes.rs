use std::collections::BTreeSet;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;
use crate::recipes_steps::{step_problems, template_problems, FileStep, SectionStep, SkillStep};
use crate::references::Origin;
use crate::validate::is_kebab_case;

/// The folder of the repository that holds the recipes of a project, beside `references/`.
pub const RECIPES_DIR: &str = "recipes";
const RECIPE_EXTENSION: &str = ".toml";
/// The folders of the recipes folder that hold templates.
pub(crate) const TEMPLATE_DIRS: [&str; 2] = ["files", "sections"];

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
    #[serde(default)]
    skills: Vec<SkillStep>,
    #[serde(default)]
    files: Vec<FileStep>,
    #[serde(default)]
    sections: Vec<SectionStep>,
}

/// A file `<id>.toml` of the catalog: the rules, the exports and the steps that prepare a
/// repository of some kind, and the recipes whose lists it adds to.
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
    pub skills: Vec<SkillStep>,
    pub files: Vec<FileStep>,
    pub sections: Vec<SectionStep>,
}

/// A file under `files/` or `sections/` of the recipes folder, whose path is from that folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Template {
    pub(crate) path: String,
    pub(crate) origin: Origin,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Recipes {
    pub(crate) recipes: Vec<Recipe>,
    pub(crate) templates: Vec<Template>,
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
        let mut recipes = Vec::new();
        let mut templates = Vec::new();
        for file in files {
            if is_template_path(&file.path) {
                templates.push(Template {
                    path: file.path.clone(),
                    origin,
                    text: file.text.clone(),
                });
            } else if let Some(recipe) = load_recipe(file, origin, &mut issues) {
                recipes.push(recipe);
            }
        }
        for recipe in &recipes {
            let exists = |path: &str| templates.iter().any(|template| template.path == path);
            let problems = template_problems(&recipe.files, &recipe.sections, &exists);
            issues.extend(
                problems
                    .into_iter()
                    .map(|message| issue(recipe_path(&recipe.id), message)),
            );
        }
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }
        recipes.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self { recipes, templates })
    }

    pub fn recipes(&self) -> &[Recipe] {
        &self.recipes
    }

    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|recipe| recipe.id == id)
    }

    /// The text of a template, by its path from the recipes folder, for the recipe `id`: the
    /// templates of a recipe are the ones that came with it, built in or of the project.
    pub fn template(&self, id: &str, path: &str) -> Option<&str> {
        let origin = self.recipe(id)?.origin;
        self.templates
            .iter()
            .find(|template| template.origin == origin && template.path == path)
            .map(|template| template.text.as_str())
    }
}

fn is_template_path(path: &str) -> bool {
    path.split_once('/').is_some_and(|(folder, rest)| {
        TEMPLATE_DIRS.contains(&folder)
            && rest
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
    })
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
            issues.push(issue(
                path,
                "unexpected file: expected <recipe>.toml, or a template under files/ or sections/",
            ));
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
    let mut problems = list_problems(&parsed);
    problems.extend(step_problems(
        &parsed.skills,
        &parsed.files,
        &parsed.sections,
    ));
    issues.extend(problems.into_iter().map(|message| issue(path, message)));
    Some(Recipe {
        id: id.to_owned(),
        origin,
        description: parsed.description,
        extends: parsed.extends,
        rules: parsed.rules,
        judgment: parsed.judgment,
        exports: parsed.exports,
        skills: parsed.skills,
        files: parsed.files,
        sections: parsed.sections,
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
