//! What `botica recipe install` does for the skills: one `npx skills add` for each source, for the
//! names that are not in `.agents/skills/`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use botica_catalog::ResolvedSkill;

use crate::recipes_check_steps::{installed, SKILLS_DIR};
use crate::recipes_install_run::failure;
use crate::recipes_install_steps::Outcome;

/// The agent that `skills add` installs for, as `scaffolding` does by default.
const AGENT: &str = "opencode";

/// The names to ask each source for, with the sources in the order they first appear.
fn calls<'a>(missing: &[&'a ResolvedSkill]) -> Vec<(&'a str, Vec<&'a str>)> {
    let mut calls: Vec<(&str, Vec<&str>)> = Vec::new();
    for skill in missing {
        match calls.iter_mut().find(|(source, _)| *source == skill.source) {
            Some((_, names)) => names.push(&skill.name),
            None => calls.push((&skill.source, vec![&skill.name])),
        }
    }
    calls
}

/// Runs `npx skills add` in the root of the project: why it failed, when it did.
fn add(root: &Path, source: &str, names: &[&str]) -> Option<String> {
    let mut command = Command::new("npx");
    command
        .args([
            "skills", "add", source, "--agent", AGENT, "--yes", "--skill",
        ])
        .args(names)
        .current_dir(root);
    failure(&mut command, &format!("`npx skills add {source}`"))
}

/// Installs the skills that are missing and says, for each skill in order, whether it was there,
/// was added, or is still missing after the call.
pub fn install(root: &Path, skills: &[ResolvedSkill]) -> Vec<Outcome> {
    let there: Vec<bool> = skills
        .iter()
        .map(|skill| installed(root, &skill.name))
        .collect();
    let missing: Vec<&ResolvedSkill> = skills
        .iter()
        .zip(&there)
        .filter(|(_, there)| !**there)
        .map(|(skill, _)| skill)
        .collect();
    let mut failures: BTreeMap<&str, String> = BTreeMap::new();
    for (source, names) in calls(&missing) {
        if let Some(reason) = add(root, source, &names) {
            failures.insert(source, reason);
        }
    }
    skills
        .iter()
        .zip(there)
        .map(|(skill, there)| {
            if there {
                Outcome::Present
            } else if installed(root, &skill.name) {
                Outcome::Added(None)
            } else {
                Outcome::Failed(
                    failures
                        .get(skill.source.as_str())
                        .cloned()
                        .unwrap_or_else(|| {
                            format!(
                                "not in `{SKILLS_DIR}/` after `npx skills add {}`",
                                skill.source
                            )
                        }),
                )
            }
        })
        .collect()
}
