use std::collections::BTreeMap;

use crate::recipes::Recipe;

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

/// The skills of the recipes in `chain`, one for each name. A name that two recipes take from
/// the same source stays where it is first; from different sources it is an error.
pub(crate) fn resolve_skills(chain: &[&Recipe]) -> Result<Vec<ResolvedSkill>, String> {
    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    let mut skills: Vec<ResolvedSkill> = Vec::new();
    for recipe in chain {
        for step in &recipe.skills {
            for name in &step.names {
                match first.get(name.as_str()) {
                    None => {
                        first.insert(name, skills.len());
                        skills.push(ResolvedSkill {
                            source: step.source.clone(),
                            name: name.clone(),
                            from: recipe.id.clone(),
                        });
                    }
                    Some(&at) if skills[at].source != step.source => {
                        return Err(format!(
                            "the skill `{name}` comes from `{}` ({}) and from `{}` ({})",
                            skills[at].source, skills[at].from, step.source, recipe.id
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
    }
    Ok(skills)
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
