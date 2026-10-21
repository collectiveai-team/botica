//! The baseline of known findings (ADR 0011): the findings that carry a measure, keyed by their
//! rule, their path from the root of the project and the qualified name of their function, with
//! the value each had when the file was written.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use botica_core::Diagnostic;
use serde::{Deserialize, Serialize};

const HEADER: &str =
    "# The known findings that `botica lint` leaves out while their value does not \
grow. `botica lint --write-baseline` writes this file.\n\n";

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default, rename = "finding")]
    findings: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    rule: String,
    path: String,
    name: String,
    value: u32,
}

type Key = (String, String, String);

pub(super) struct Baseline {
    values: BTreeMap<Key, u32>,
}

/// The path of the baseline that the configuration names, from the root of the project.
pub(super) fn path_of(root: &Path, configured: Option<&str>) -> Option<PathBuf> {
    configured.map(|path| root.join(path))
}

impl Baseline {
    pub(super) fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| {
            format!(
                "cannot read the baseline `{}`: `botica lint --write-baseline` writes it",
                path.display()
            )
        })?;
        let file: File = toml::from_str(&text)
            .with_context(|| format!("the baseline `{}` is not valid", path.display()))?;
        let values = file
            .findings
            .into_iter()
            .map(|entry| ((entry.rule, entry.path, entry.name), entry.value))
            .collect();
        Ok(Self { values })
    }

    /// Leaves out the findings that the baseline holds with a value at least as high, and says in
    /// the message of the others which value it holds.
    pub(super) fn apply(&self, prefix: Option<&str>, diagnostics: &mut Vec<Diagnostic>) {
        diagnostics.retain_mut(|diagnostic| {
            let Some(key) = key_of(prefix, diagnostic) else {
                return true;
            };
            let Some(&known) = self.values.get(&key) else {
                return true;
            };
            let value = diagnostic
                .measure
                .as_ref()
                .map_or(0, |measure| measure.value);
            if value <= known {
                return false;
            }
            diagnostic.message = format!("{} (the baseline holds {known})", diagnostic.message);
            true
        });
    }
}

/// Writes the findings that carry a measure to `path`, and returns how many it wrote.
pub(super) fn write(
    path: &Path,
    prefix: Option<&str>,
    diagnostics: &[Diagnostic],
) -> Result<usize> {
    let mut findings: Vec<Entry> = diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let (rule, path, name) = key_of(prefix, diagnostic)?;
            let value = diagnostic.measure.as_ref()?.value;
            Some(Entry {
                rule,
                path,
                name,
                value,
            })
        })
        .collect();
    findings.sort_by(|a, b| (&a.path, &a.rule, &a.name).cmp(&(&b.path, &b.rule, &b.name)));
    let count = findings.len();
    let text = toml::to_string(&File { findings }).context("cannot write the baseline")?;
    std::fs::write(path, format!("{HEADER}{text}"))
        .with_context(|| format!("cannot write the baseline `{}`", path.display()))?;
    Ok(count)
}

/// `--write-baseline` reads the whole project: from its root and with no paths, so the file it
/// writes keeps no finding of a part it did not read.
pub(super) fn check_whole_project(prefix: Option<&str>, paths: &[PathBuf]) -> Result<()> {
    if prefix.is_some() || !paths.is_empty() {
        bail!("`--write-baseline` runs at the root of the project and takes no paths");
    }
    Ok(())
}

fn key_of(prefix: Option<&str>, diagnostic: &Diagnostic) -> Option<Key> {
    let measure = diagnostic.measure.as_ref()?;
    Some((
        diagnostic.rule.clone(),
        from_root(prefix, &diagnostic.path),
        measure.name.clone(),
    ))
}

/// A path seen from the directory `prefix` inside the project, from its root.
fn from_root(prefix: Option<&str>, path: &str) -> String {
    let mut parts: Vec<&str> = prefix
        .map(|prefix| prefix.split('/').collect())
        .unwrap_or_default();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}
