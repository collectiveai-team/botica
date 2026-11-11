//! The files of the history, read as ADR 0019 decision 9.4 states: regular files named
//! `runs.jsonl` and `findings.jsonl`, each read on its own, within the size limits.

use super::lines::{self, Finding, Run};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const FILE_MAX: u64 = 50 * 1024 * 1024;
const TOTAL_MAX: u64 = 2 * 1024 * 1024 * 1024;

/// The source of the local history, which `botica` wrote in this checkout.
pub const LOCAL: usize = 0;

/// Where a group of lines comes from: the local history, or the CI run that uploaded them.
#[derive(Debug, Clone)]
pub struct Source {
    pub label: String,
    /// The local history is trusted for the person who wrote it; a downloaded run is trusted when
    /// its `source.json` says so.
    pub trusted: bool,
}

/// Every line that passed its checks, with what the reader skipped.
#[derive(Debug, Default)]
pub struct History {
    pub sources: Vec<Source>,
    pub runs: Vec<Run>,
    pub findings: Vec<Finding>,
    pub notes: Vec<String>,
    pub skipped_lines: usize,
}

impl History {
    pub fn trusted(&self, source: usize) -> bool {
        self.sources[source].trusted
    }
}

/// Reads `.botica/history/` under `root`, and each folder `<id>` of the folders in `from`.
pub fn load(root: &Path, from: &[std::path::PathBuf]) -> Result<History> {
    let mut reader = Reader::default();
    reader.history.sources.push(Source {
        label: "local history".to_owned(),
        trusted: true,
    });
    reader.add(&root.join(".botica").join("history"), LOCAL);
    for dir in from {
        reader.add_downloaded(dir)?;
    }
    Ok(reader.finish())
}

#[derive(Default)]
struct Reader {
    history: History,
    used: u64,
}

impl Reader {
    fn add_downloaded(&mut self, dir: &Path) -> Result<()> {
        let listed = std::fs::read_dir(dir)
            .with_context(|| format!("cannot read the folder {}", dir.display()))?;
        let mut ids: Vec<_> = listed.filter_map(Result::ok).collect();
        ids.sort_by_key(std::fs::DirEntry::file_name);
        for entry in ids {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if !is_dir || name.len() > 256 || name.chars().any(char::is_control) {
                self.note(format!(
                    "{} is not a folder of a CI run: skipped",
                    shown(&entry.path())
                ));
                continue;
            }
            let trusted = self.trusted(&entry.path().join("source.json"), &name);
            self.history.sources.push(Source {
                label: format!("CI run {name}"),
                trusted,
            });
            let source = self.history.sources.len() - 1;
            self.add(&entry.path(), source);
        }
        Ok(())
    }

    /// The `trusted` of `source.json`, written by `botica history fetch` from the answer of the
    /// API; a run with no readable file is not trusted.
    fn trusted(&mut self, path: &Path, name: &str) -> bool {
        let Some(bytes) = self.file(path) else {
            self.note(format!("CI run {name} has no source.json: not trusted"));
            return false;
        };
        let trusted = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| value["trusted"].as_bool());
        trusted.unwrap_or_else(|| {
            self.note(format!(
                "source.json of CI run {name} does not parse: not trusted"
            ));
            false
        })
    }

    fn add(&mut self, dir: &Path, source: usize) {
        if let Some(bytes) = self.file(&dir.join("runs.jsonl")) {
            for line in non_blank(&bytes) {
                match lines::run(source, line) {
                    Some(run) => self.history.runs.push(run),
                    None => self.history.skipped_lines += 1,
                }
            }
        }
        if let Some(bytes) = self.file(&dir.join("findings.jsonl")) {
            for line in non_blank(&bytes) {
                match lines::finding(source, line) {
                    Some(finding) => self.history.findings.push(finding),
                    None => self.history.skipped_lines += 1,
                }
            }
        }
    }

    /// The bytes of a regular file within the limits; a file past a limit or that is not a regular
    /// file is named in the notes, and one that is missing is not.
    fn file(&mut self, path: &Path) -> Option<Vec<u8>> {
        let found = match std::fs::symlink_metadata(path) {
            Ok(found) => found,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                self.note(format!("cannot read {}: {error}", shown(path)));
                return None;
            }
        };
        if !found.is_file() {
            self.note(format!("{} is not a regular file: skipped", shown(path)));
            return None;
        }
        if found.len() > FILE_MAX || self.used.saturating_add(found.len()) > TOTAL_MAX {
            self.note(format!(
                "{} is past the size limit of the reader: skipped, so the history is cut",
                shown(path)
            ));
            return None;
        }
        let mut bytes = Vec::new();
        let read =
            File::open(path).and_then(|file| file.take(FILE_MAX + 1).read_to_end(&mut bytes));
        if let Err(error) = read {
            self.note(format!("cannot read {}: {error}", shown(path)));
            return None;
        }
        self.used += bytes.len() as u64;
        Some(bytes)
    }

    fn note(&mut self, text: String) {
        self.history.notes.push(text);
    }

    /// Drops what decision 2 says to ignore: a run named twice with different lines, and a
    /// finding whose run has no line; a line repeated in one source is read once.
    fn finish(mut self) -> History {
        let history = &mut self.history;
        let mut first: HashMap<(usize, String), usize> = HashMap::new();
        let mut clashing: HashSet<(usize, String)> = HashSet::new();
        for (position, run) in history.runs.iter().enumerate() {
            let key = (run.source, run.id.clone());
            match first.get(&key) {
                Some(&earlier) if history.runs[earlier] != *run => {
                    clashing.insert(key);
                }
                Some(_) => {}
                None => {
                    first.insert(key, position);
                }
            }
        }
        for (source, id) in &clashing {
            let label = history.sources[*source].label.clone();
            history.notes.push(format!(
                "two different run lines name run {id} of the {label}: both are ignored"
            ));
        }
        let mut kept = HashSet::new();
        let runs = std::mem::take(&mut history.runs);
        history.runs = runs
            .into_iter()
            .filter(|run| {
                let key = (run.source, run.id.clone());
                !clashing.contains(&key) && kept.insert(key)
            })
            .collect();
        let named: HashSet<(usize, &str)> = history
            .runs
            .iter()
            .map(|run| (run.source, run.id.as_str()))
            .collect();
        let mut seen = HashSet::new();
        let findings = std::mem::take(&mut history.findings);
        history.findings = findings
            .into_iter()
            .filter(|finding| {
                named.contains(&(finding.source, finding.run_id.as_str()))
                    && seen.insert((
                        finding.source,
                        finding.run_id.clone(),
                        finding.fingerprint.clone(),
                        finding.occurrence,
                    ))
            })
            .collect();
        self.history
    }
}

fn non_blank(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
}

/// A path for a message, with no control character in it.
fn shown(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .map(|found| if found.is_control() { '?' } else { found })
        .collect()
}
