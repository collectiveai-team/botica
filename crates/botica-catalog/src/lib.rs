mod dir;
mod ecosystem;
mod embedded;
mod extension;
mod issue;
mod load;
mod model;
mod origin;
mod project;
mod question;
mod readme;
mod recipes;
mod recipes_merge;
mod recipes_resolve;
mod recipes_steps;
mod recipes_steps_resolve;
mod references;
mod references_check;
mod references_merge;
mod sources;
mod validate;

pub use validate::is_kebab_case;

pub use dir::{
    project_catalog_dirs, read_catalog_files, read_files, read_project_files, read_recipes,
    read_references,
};
pub use ecosystem::Ecosystem;
pub use issue::{CatalogError, CatalogIssue};
pub use load::{Catalog, SourceFile, CATALOG_FILE, HISTORY_FILE, OWN_RULES_DIR};
pub use model::{
    Event, Family, Implementation, Kind, Lifecycle, Needs, OptionSpec, OptionType, Rule, RunOn,
    Runtime, Scope, Severity, Status, When, CONFIG_DIR,
};
pub use question::{BooleanCriteria, Choices, Question};
pub use readme::Section;
pub use recipes::{Recipe, Recipes, AGENTS, EXPORTS, RECIPES_DIR};
pub use recipes_resolve::{RecipeItem, ResolvedRecipe};
pub use recipes_steps::{
    normalized_package, FileStep, GitignoreStep, LinkStep, SectionStep, SkillStep, ToolStep,
    DEFAULT_TOOL_GROUP, TOOL_MANAGERS, UNIGNORE_WHITELIST,
};
pub use recipes_steps_resolve::{
    ResolvedFile, ResolvedLink, ResolvedSection, ResolvedSkill, ResolvedTool,
};
pub use references::{Origin, Reference, References, Topic, CODE_DIR, REFERENCES_DIR, TOPIC_FILE};
