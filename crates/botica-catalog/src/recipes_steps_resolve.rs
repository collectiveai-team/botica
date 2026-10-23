use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::recipes::Recipe;
use crate::recipes_resolve::RecipeItem;
use crate::recipes_steps::{
    requirement_package, same_requirement, GitignoreStep, DEFAULT_TOOL_GROUP,
};

/// A skill to install and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSkill {
    pub source: String,
    pub name: String,
    pub from: String,
}

/// A file to add and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFile {
    pub path: String,
    pub template: String,
    pub from: String,
}

/// A section to add and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSection {
    pub path: String,
    pub marker: String,
    pub template: String,
    pub from: String,
}

/// A link to add and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    pub path: String,
    pub target: String,
    pub from: String,
}

/// A package to add, with the manager and the group, and the recipe whose file lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTool {
    pub manager: String,
    pub group: String,
    pub package: String,
    pub from: String,
}

/// The skills of the recipes in `chain`, one for each name, with a warning for each skill that a
/// recipe takes from another source than the recipe before it. A name that two recipes take from
/// the same source stays where it is first; from different sources the later recipe takes it
/// over and keeps the place of the skill.
pub(crate) fn resolve_skills(chain: &[&Recipe]) -> (Vec<ResolvedSkill>, Vec<String>) {
    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    let mut skills: Vec<ResolvedSkill> = Vec::new();
    let mut warnings = Vec::new();
    for recipe in chain {
        for step in &recipe.skills {
            for name in &step.names {
                let Some(&at) = first.get(name.as_str()) else {
                    first.insert(name, skills.len());
                    skills.push(ResolvedSkill {
                        source: step.source.clone(),
                        name: name.clone(),
                        from: recipe.id.clone(),
                    });
                    continue;
                };
                if skills[at].source != step.source {
                    warnings.push(format!(
                        "the skill `{name}` comes from `{}` ({}), which replaces `{}` ({})",
                        step.source, recipe.id, skills[at].source, skills[at].from
                    ));
                    skills[at].source = step.source.clone();
                    skills[at].from = recipe.id.clone();
                }
            }
        }
    }
    (skills, warnings)
}

/// The files of the recipes in `chain`; a file that two recipes list the same stays where it is
/// first.
pub(crate) fn resolve_files(chain: &[&Recipe]) -> Vec<ResolvedFile> {
    let mut files: Vec<ResolvedFile> = Vec::new();
    for recipe in chain {
        for step in &recipe.files {
            let listed = files
                .iter()
                .any(|f| f.path == step.path && f.template == step.template);
            if !listed {
                files.push(ResolvedFile {
                    path: step.path.clone(),
                    template: step.template.clone(),
                    from: recipe.id.clone(),
                });
            }
        }
    }
    files
}

/// The sections of the recipes in `chain`; a section that two recipes list the same stays where
/// it is first.
pub(crate) fn resolve_sections(chain: &[&Recipe]) -> Vec<ResolvedSection> {
    let mut sections: Vec<ResolvedSection> = Vec::new();
    for recipe in chain {
        for step in &recipe.sections {
            let listed = sections.iter().any(|s| {
                s.path == step.path && s.marker == step.marker && s.template == step.template
            });
            if !listed {
                sections.push(ResolvedSection {
                    path: step.path.clone(),
                    marker: step.marker.clone(),
                    template: step.template.clone(),
                    from: recipe.id.clone(),
                });
            }
        }
    }
    sections
}

/// The lines of the `gitignore` steps of the recipes in `chain` that `list` picks, each one once,
/// where it is first.
pub(crate) fn resolve_gitignore(
    chain: &[&Recipe],
    list: fn(&GitignoreStep) -> &Vec<String>,
) -> Vec<RecipeItem> {
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for recipe in chain {
        for line in recipe.gitignore.iter().flat_map(list) {
            if seen.insert(line.as_str()) {
                items.push(RecipeItem {
                    id: line.clone(),
                    from: recipe.id.clone(),
                });
            }
        }
    }
    items
}

/// The links of the recipes in `chain`; a link that two recipes list the same stays where it is
/// first.
pub(crate) fn resolve_links(chain: &[&Recipe]) -> Vec<ResolvedLink> {
    let mut links: Vec<ResolvedLink> = Vec::new();
    for recipe in chain {
        for step in &recipe.links {
            let listed = links
                .iter()
                .any(|l| l.path == step.path && l.target == step.target);
            if !listed {
                links.push(ResolvedLink {
                    path: step.path.clone(),
                    target: step.target.clone(),
                    from: recipe.id.clone(),
                });
            }
        }
    }
    links
}

/// A package of a tool that two recipes of a chain ask for with different requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolConflict {
    pub manager: String,
    pub group: String,
    /// The name of the package, normalized as `uv` compares names: `my-pkg`.
    pub package: String,
    /// The requirement that comes first in the chain, and the recipe that asks for it.
    pub first: RecipeItem,
    /// The requirement of a later recipe of the chain, and that recipe.
    pub then: RecipeItem,
}

impl fmt::Display for ToolConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` asks for `{}` and `{}` for `{}`: a package has one requirement across the recipes",
            self.then.from, self.then.id, self.first.from, self.first.id
        )
    }
}

/// The packages of the `tools` steps of the recipes in `chain`, one entry for each manager, group
/// and package, where it is first. Two recipes of the chain that ask for the same package with
/// different requirements are an error: which one wins would depend on the order of `extends`.
pub(crate) fn resolve_tools(chain: &[&Recipe]) -> Result<Vec<ResolvedTool>, String> {
    let (tools, conflicts) = gather_tools(chain);
    match conflicts.first() {
        Some(conflict) => Err(conflict.to_string()),
        None => Ok(tools),
    }
}

/// Each package of the `tools` steps of the recipes in `chain` that a recipe asks for with
/// another requirement than the first recipe of the chain that asks for it, in the order of the
/// chain.
pub(crate) fn tool_conflicts(chain: &[&Recipe]) -> Vec<ToolConflict> {
    gather_tools(chain).1
}

fn gather_tools(chain: &[&Recipe]) -> (Vec<ResolvedTool>, Vec<ToolConflict>) {
    let mut tools: Vec<ResolvedTool> = Vec::new();
    let mut conflicts = Vec::new();
    for recipe in chain {
        for step in &recipe.tools {
            let group = step.group.as_deref().unwrap_or(DEFAULT_TOOL_GROUP);
            for package in &step.packages {
                let name = requirement_package(package);
                let listed = tools.iter().find(|t| {
                    t.manager == step.manager
                        && t.group == group
                        && requirement_package(&t.package) == name
                });
                match listed {
                    Some(first) if !same_requirement(&first.package, package) => {
                        conflicts.push(ToolConflict {
                            manager: step.manager.clone(),
                            group: group.to_owned(),
                            package: name,
                            first: RecipeItem {
                                id: first.package.clone(),
                                from: first.from.clone(),
                            },
                            then: RecipeItem {
                                id: package.clone(),
                                from: recipe.id.clone(),
                            },
                        });
                    }
                    Some(_) => {}
                    None => tools.push(ResolvedTool {
                        manager: step.manager.clone(),
                        group: group.to_owned(),
                        package: package.clone(),
                        from: recipe.id.clone(),
                    }),
                }
            }
        }
    }
    (tools, conflicts)
}
