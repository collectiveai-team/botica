//! What `botica recipe install` does for the `gitignore` step: it adds the lines that `.gitignore`
//! lacks and removes the lines of the whitelist (ADR 0001), and edits nothing else.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

use crate::recipes_gitignore::{has_line, has_literal, Gitignore};
use crate::recipes_install_steps::Outcome;

const FILE: &str = ".gitignore";

/// Writes `.gitignore` with the lines when it does not exist, and appends the ones it lacks when
/// it does. The outcome of each line, in order.
pub fn add_lines(root: &Path, lines: &[&str]) -> Result<Vec<Outcome>> {
    let full = root.join(FILE);
    let text = match Gitignore::read(root) {
        Gitignore::Unusable(reason) => {
            return Ok(lines
                .iter()
                .map(|_| Outcome::Pending(reason.clone()))
                .collect());
        }
        Gitignore::Missing => String::new(),
        Gitignore::Text(text) => text,
    };
    let missing: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !has_line(&text, line))
        .collect();
    if !missing.is_empty() {
        let separator = if text.is_empty() || text.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full)
            .with_context(|| format!("cannot open `{}`", full.display()))?;
        writeln!(file, "{separator}{}", missing.join("\n"))
            .with_context(|| format!("cannot write `{}`", full.display()))?;
    }
    Ok(lines
        .iter()
        .map(|line| {
            if missing.contains(line) {
                Outcome::Added(None)
            } else {
                Outcome::Present
            }
        })
        .collect())
}

/// Removes each line of `.gitignore` that is `line` between white space, and nothing else: the
/// endings of the other lines stay as they are.
pub fn unignore(root: &Path, line: &str) -> Result<Outcome> {
    let text = match Gitignore::read(root) {
        Gitignore::Missing => return Ok(Outcome::Present),
        Gitignore::Unusable(reason) => return Ok(Outcome::Pending(reason)),
        Gitignore::Text(text) => text,
    };
    if !has_literal(&text, line) {
        return Ok(Outcome::Present);
    }
    let kept: String = text
        .split_inclusive('\n')
        .filter(|candidate| candidate.trim() != line)
        .collect();
    let full = root.join(FILE);
    fs::write(&full, kept).with_context(|| format!("cannot write `{}`", full.display()))?;
    Ok(Outcome::Edited("removed the line from .gitignore"))
}
