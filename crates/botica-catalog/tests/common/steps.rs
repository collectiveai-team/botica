//! What the tests of the steps of a recipe share.

use std::collections::BTreeSet;

use botica_catalog::{CatalogError, Recipes, SourceFile};

use super::file;

pub fn recipe(id: &str, toml: &str) -> SourceFile {
    file(&format!("{id}.toml"), toml)
}

/// The recipes with a file for each template that they name.
pub fn project(mut files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    let templates: BTreeSet<String> = files
        .iter()
        .flat_map(|file| file.text.lines())
        .filter_map(|line| line.strip_prefix("template = \""))
        .map(|rest| rest.trim_end_matches('"').to_owned())
        .filter(|path| path.starts_with("files/") || path.starts_with("sections/"))
        .collect();
    files.extend(templates.iter().map(|path| file(path, "template\n")));
    Recipes::from_files(&files)
}

pub const STEPS: &str = r###"
[[skills]]
source = "mattpocock/skills#v1.2.3"
names = ["tdd", "grilling"]

[[skills]]
source = "collectiveai-team/botica"
names = ["handoff"]

[[files]]
path = "prek.toml"
template = "files/prek-base.toml"

[[sections]]
path = "AGENTS.md"
marker = "## Botica rules"
template = "sections/botica-rules.md"
"###;
