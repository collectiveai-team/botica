//! The selection step of `botica recipe install`: the rules of the recipes go into the `select` of
//! the configuration, which `botica lint`, the hooks of prek and the editor read.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::recipes_install_steps::Outcome;

pub const BOTICA_TOML: &str = "botica.toml";
const PYPROJECT: &str = "pyproject.toml";

/// The text of a new `botica.toml` that selects `rules`.
fn text(names: &[String], rules: &[&str]) -> String {
    let mut text = format!(
        "# The rules of the recipes {}, written by `botica recipe install`. `botica lint`, its hooks\n\
         # in prek.toml and the editor read this list, and a rule that botica adds later runs once it\n\
         # is listed here. `botica recipe check` compares it with the recipes.\n\
         select = [\n",
        names
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    for rule in rules {
        let _ = writeln!(text, "  \"{rule}\",");
    }
    text.push_str("]\n");
    text
}

/// The configuration of botica that the project already has: the file and its table.
fn existing(root: &Path) -> Option<(&'static str, Option<toml::Table>)> {
    let read = |name: &str| {
        fs::read_to_string(root.join(name))
            .ok()
            .and_then(|text| text.parse::<toml::Table>().ok())
    };
    if root.join(BOTICA_TOML).exists() {
        return Some((BOTICA_TOML, read(BOTICA_TOML)));
    }
    let pyproject = read(PYPROJECT)?;
    let table = pyproject.get("tool")?.get("botica")?.as_table()?.clone();
    Some((PYPROJECT, Some(table)))
}

/// Writes `botica.toml` with the rules when the project has no configuration of botica. One that
/// exists is `ok` when its `select` lists every rule, and pending otherwise.
pub fn add(root: &Path, names: &[String], rules: &[&str]) -> Result<Outcome> {
    let Some((file, table)) = existing(root) else {
        let path = root.join(BOTICA_TOML);
        fs::write(&path, text(names, rules))
            .with_context(|| format!("cannot write `{}`", path.display()))?;
        return Ok(Outcome::Added(None));
    };
    let Some(table) = table else {
        return Ok(Outcome::Pending(format!("`{file}` is not valid TOML")));
    };
    let selected: Vec<&str> = table
        .get("select")
        .and_then(toml::Value::as_array)
        .map(|select| select.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    let missing: Vec<&str> = rules
        .iter()
        .copied()
        .filter(|rule| !selected.contains(rule))
        .collect();
    Ok(if missing.is_empty() {
        Outcome::Present
    } else {
        Outcome::Pending(format!(
            "the configuration in `{file}` exists and its `select` lacks {}",
            missing.join(", ")
        ))
    })
}
