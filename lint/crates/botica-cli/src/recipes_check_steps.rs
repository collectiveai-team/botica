//! What `botica recipe check` finds out about the files, the sections and the skills of a recipe.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::recipes_gitignore::{has_line, has_literal, Gitignore};

const LOCK: &str = "skills-lock.json";
pub const SKILLS_DIR: &str = ".agents/skills";

/// Why the file `path`, from the root of the project, is not there: `None` when it is a file.
pub fn file(root: &Path, path: &str) -> Option<String> {
    let full = root.join(path);
    if full.is_file() {
        None
    } else if full.exists() {
        Some("is not a file".to_owned())
    } else {
        Some("does not exist".to_owned())
    }
}

/// Why the file of a section does not have its marker. The marker is searched for anywhere in the
/// text.
pub fn section(root: &Path, path: &str, marker: &str) -> Option<String> {
    if let Some(problem) = file(root, path) {
        return Some(problem);
    }
    match fs::read_to_string(root.join(path)) {
        Err(error) => Some(format!("cannot read the file: {error}")),
        Ok(text) => (!text.contains(marker)).then(|| "marker not in the file".to_owned()),
    }
}

/// The names that `skills-lock.json` declares, or why it cannot say.
pub enum Lock {
    Missing,
    NotAManifest,
    Names(BTreeSet<String>),
}

impl Lock {
    pub fn read(root: &Path) -> Self {
        let Ok(text) = fs::read_to_string(root.join(LOCK)) else {
            return Self::Missing;
        };
        let skills = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| value.get("skills").cloned());
        match skills {
            Some(serde_json::Value::Object(map)) => Self::Names(map.keys().cloned().collect()),
            _ => Self::NotAManifest,
        }
    }
}

/// Why the skill `name` is not set up: it is not in the lock, or not in `.agents/skills/`.
pub fn skill(root: &Path, lock: &Lock, name: &str) -> Option<String> {
    let mut problems = Vec::new();
    match lock {
        Lock::Missing => problems.push(format!("no `{LOCK}`")),
        Lock::NotAManifest => problems.push(format!("`{LOCK}` is not a manifest of skills")),
        Lock::Names(names) if !names.contains(name) => {
            problems.push(format!("not in `{LOCK}`"));
        }
        Lock::Names(_) => {}
    }
    if !installed(root, name) {
        problems.push(format!("not in `{SKILLS_DIR}/`"));
    }
    (!problems.is_empty()).then(|| problems.join(" and "))
}

/// Whether `.agents/skills/<name>/SKILL.md` is a file.
pub fn installed(root: &Path, name: &str) -> bool {
    root.join(SKILLS_DIR).join(name).join("SKILL.md").is_file()
}

/// Why `.gitignore` does not have `line`.
pub fn gitignore_line(root: &Path, line: &str) -> Option<String> {
    match Gitignore::read(root) {
        Gitignore::Missing => Some("no .gitignore".to_owned()),
        Gitignore::Unusable(reason) => Some(reason),
        Gitignore::Text(text) => (!has_line(&text, line)).then(|| "not in .gitignore".to_owned()),
    }
}

/// Why `.gitignore` still ignores `line`, as a line of its own, or cannot say.
pub fn unignored(root: &Path, line: &str) -> Option<String> {
    match Gitignore::read(root) {
        Gitignore::Missing => None,
        Gitignore::Unusable(reason) => Some(reason),
        Gitignore::Text(text) => {
            has_literal(&text, line).then(|| ".gitignore still has the line".to_owned())
        }
    }
}

/// What is at the path of a link.
pub enum LinkState {
    /// A link whose text is the target.
    Right,
    Missing,
    /// Something else: a file, a folder, or a link to another target.
    Other(String),
}

pub fn link_state(root: &Path, path: &str, target: &str) -> LinkState {
    let full = root.join(path);
    match fs::read_link(&full) {
        Ok(found) if found == Path::new(target) => LinkState::Right,
        Ok(found) => LinkState::Other(format!(
            "is a link to `{}`, not to `{target}`",
            found.display()
        )),
        Err(_) if fs::symlink_metadata(&full).is_ok() => {
            LinkState::Other("is not a link".to_owned())
        }
        Err(_) => LinkState::Missing,
    }
}

/// Why the link `path` is not there: it does not exist, or it is not a link to `target`.
pub fn link(root: &Path, path: &str, target: &str) -> Option<String> {
    match link_state(root, path, target) {
        LinkState::Right => None,
        LinkState::Missing => Some("does not exist".to_owned()),
        LinkState::Other(problem) => Some(problem),
    }
}
