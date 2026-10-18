use std::collections::BTreeSet;

use crate::issue::{issue, CatalogIssue};
use crate::recipes::{recipe_path, Recipe, Recipes};
use crate::recipes_steps_resolve::{
    resolve_files, resolve_gitignore, resolve_links, resolve_sections, resolve_skills,
    resolve_tools, ResolvedFile, ResolvedLink, ResolvedSection, ResolvedSkill, ResolvedTool,
};

/// An entry of a resolved recipe and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeItem {
    pub id: String,
    pub from: String,
}

/// A recipe with the lists of the recipes it extends added before its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRecipe {
    pub id: String,
    /// The recipes that contribute, each after the ones it extends and the recipe itself last.
    pub chain: Vec<String>,
    pub rules: Vec<RecipeItem>,
    pub judgment: Vec<RecipeItem>,
    pub exports: Vec<RecipeItem>,
    pub skills: Vec<ResolvedSkill>,
    pub files: Vec<ResolvedFile>,
    pub sections: Vec<ResolvedSection>,
    pub links: Vec<ResolvedLink>,
    pub tools: Vec<ResolvedTool>,
    /// The lines to add to `.gitignore`.
    pub gitignore: Vec<RecipeItem>,
    /// The lines to remove from `.gitignore`.
    pub unignore: Vec<RecipeItem>,
    /// What the resolution changed without failing, one message for each.
    pub warnings: Vec<String>,
}

impl Recipes {
    /// The recipe `id` with what it extends: each recipe contributes once, after the recipes it
    /// extends, and an entry that several recipes list stays where it is first.
    pub fn resolve(&self, id: &str) -> Result<ResolvedRecipe, String> {
        if self.recipe(id).is_none() {
            return Err(format!(
                "no recipe `{id}` (`botica recipe list` lists them)"
            ));
        }
        let mut chain = Vec::new();
        self.visit(id, &mut Vec::new(), &mut chain)?;
        let contribute = |list: fn(&Recipe) -> &Vec<String>| {
            let mut seen = BTreeSet::new();
            let mut items = Vec::new();
            for recipe in chain.iter().filter_map(|id| self.recipe(id)) {
                for entry in list(recipe) {
                    if seen.insert(entry.clone()) {
                        items.push(RecipeItem {
                            id: entry.clone(),
                            from: recipe.id.clone(),
                        });
                    }
                }
            }
            items
        };
        let recipes: Vec<&Recipe> = chain.iter().filter_map(|id| self.recipe(id)).collect();
        let (skills, warnings) = resolve_skills(&recipes);
        Ok(ResolvedRecipe {
            id: id.to_owned(),
            rules: contribute(|recipe| &recipe.rules),
            judgment: contribute(|recipe| &recipe.judgment),
            exports: contribute(|recipe| &recipe.exports),
            skills,
            warnings,
            files: resolve_files(&recipes),
            sections: resolve_sections(&recipes),
            links: resolve_links(&recipes),
            tools: resolve_tools(&recipes),
            gitignore: resolve_gitignore(&recipes, |step| &step.lines),
            unignore: resolve_gitignore(&recipes, |step| &step.unignore),
            chain,
        })
    }

    fn visit(
        &self,
        id: &str,
        path: &mut Vec<String>,
        chain: &mut Vec<String>,
    ) -> Result<(), String> {
        if chain.iter().any(|done| done == id) {
            return Ok(());
        }
        if path.iter().any(|open| open == id) {
            return Err(format!("`{id}` extends itself"));
        }
        let recipe = self.recipe(id).ok_or_else(|| {
            let parent = path.last().map_or("a recipe", String::as_str);
            format!("`{parent}` extends `{id}`, which is not a recipe")
        })?;
        path.push(id.to_owned());
        for parent in &recipe.extends {
            self.visit(parent, path, chain)?;
        }
        path.pop();
        chain.push(id.to_owned());
        Ok(())
    }

    /// The recipes that extend one that does not exist, and the cycles of `extends`.
    pub(crate) fn extends_problems(&self) -> Vec<CatalogIssue> {
        let mut issues = Vec::new();
        for recipe in &self.recipes {
            for parent in &recipe.extends {
                if self.recipe(parent).is_none() {
                    issues.push(issue(
                        recipe_path(&recipe.id),
                        format!("extends `{parent}`, which is not a recipe"),
                    ));
                }
            }
        }
        let mut done = BTreeSet::new();
        for recipe in &self.recipes {
            self.find_cycle(&recipe.id, &mut Vec::new(), &mut done, &mut issues);
        }
        issues
    }

    fn find_cycle(
        &self,
        id: &str,
        path: &mut Vec<String>,
        done: &mut BTreeSet<String>,
        issues: &mut Vec<CatalogIssue>,
    ) {
        if done.contains(id) {
            return;
        }
        if let Some(start) = path.iter().position(|open| open == id) {
            let mut cycle: Vec<&str> = path[start..].iter().map(String::as_str).collect();
            cycle.push(id);
            let last = path.last().map_or(id, String::as_str);
            issues.push(issue(
                recipe_path(last),
                format!("extends itself: {}", cycle.join(" -> ")),
            ));
            return;
        }
        let Some(recipe) = self.recipe(id) else {
            return;
        };
        path.push(id.to_owned());
        for parent in &recipe.extends {
            self.find_cycle(parent, path, done, issues);
        }
        path.pop();
        done.insert(id.to_owned());
    }
}
