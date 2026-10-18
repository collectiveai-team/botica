//! What `botica recipe install` does for the skills: one `npx skills add` for each source, for the
//! names that are not in `.agents/skills/`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use botica_catalog::ResolvedSkill;

use crate::recipes_check_steps::{installed, SKILLS_DIR};
use crate::recipes_install_json::skipped;
use crate::recipes_install_run::run;
use crate::recipes_install_steps::Outcome;

/// The agent that `skills add` installs for when no recipe names one, as `scaffolding` does.
const DEFAULT_AGENT: &str = "opencode";

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

/// What the call for a source did: why it failed, when it did, and the skills it skipped.
struct Call {
    failure: Option<String>,
    skipped: BTreeMap<String, String>,
}

/// Runs `npx skills add` in the root of the project.
fn add(root: &Path, source: &str, names: &[&str], agents: &[&str]) -> Call {
    let mut command = Command::new("npx");
    command
        .args(["skills", "add", source, "--agent"])
        .args(agents)
        .args(["--yes", "--json", "--skill"])
        .args(names)
        .current_dir(root);
    let ran = run(&mut command, &format!("`npx skills add {source}`"));
    Call {
        failure: ran.failure,
        skipped: skipped(&ran.stdout),
    }
}

/// Installs the skills that are missing for `agents` (`opencode` when there are none) and says, for each skill in order, whether it was there,
/// was added, or is still missing after the call.
pub fn install(root: &Path, skills: &[ResolvedSkill], agents: &[&str]) -> Vec<Outcome> {
    let agents = if agents.is_empty() {
        &[DEFAULT_AGENT]
    } else {
        agents
    };
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
    let mut made: BTreeMap<&str, Call> = BTreeMap::new();
    for (source, names) in calls(&missing) {
        made.insert(source, add(root, source, &names, agents));
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
                Outcome::Failed(why_missing(skill, made.get(skill.source.as_str())))
            }
        })
        .collect()
}

/// Why a skill is not in `.agents/skills/` after the call for its source: the call skipped it, the
/// call failed, or it did not install it.
fn why_missing(skill: &ResolvedSkill, call: Option<&Call>) -> String {
    let source = &skill.source;
    if let Some(reason) = call.and_then(|call| call.skipped.get(&skill.name)) {
        return format!("`npx skills add {source}` skipped it: {reason}");
    }
    call.and_then(|call| call.failure.clone())
        .unwrap_or_else(|| format!("not in `{SKILLS_DIR}/` after `npx skills add {source}`"))
}
