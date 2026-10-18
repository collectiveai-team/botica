use std::collections::{BTreeMap, BTreeSet};

use crate::recipes::Recipe;
use crate::recipes_resolve::RecipeItem;
use crate::recipes_steps::GitignoreStep;

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
