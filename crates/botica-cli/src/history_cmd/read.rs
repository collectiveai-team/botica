//! The files of the history, read as ADR 0019 decision 9.4 states: regular files named
//! `runs.jsonl` and `findings.jsonl`, each read on its own, within the size limits. A run that
//! started before the window is dropped as it is read, and a finding of a run that was dropped
//! never reaches memory.

use super::lines::{self, Finding, Run};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

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

/// Every line of the window that passed its checks, with what the reader skipped.
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

/// Reads `.botica/history/` under `root`, and each folder `<id>` of the folders in `from`, keeping
/// the runs that started at `since` (Unix seconds) or later.
pub fn load(root: &Path, from: &[PathBuf], since: u64) -> Result<History> {
    let mut reader = Reader {
        since,
        ..Reader::default()
    };
    reader.history.sources.push(Source {
        label: "local history".to_owned(),
        trusted: true,
    });
    reader.add_local(root);
    for dir in from {
        reader.add_downloaded(dir)?;
    }
    Ok(reader.finish())
}

#[derive(Default)]
struct Reader {
    history: History,
    since: u64,
    used: u64,
    /// The folders that hold a `runs.jsonl` and a `findings.jsonl`, with their source.
    folders: Vec<(PathBuf, usize)>,
    /// The source of each CI run, by the name of its folder, so that a run found under two
    /// `--from` folders is one source.
    by_name: HashMap<String, usize>,
}

impl Reader {
    fn add_local(&mut self, root: &Path) {
        let dot = root.join(".botica");
        let dir = dot.join("history");
        if self.is_plain_dir(&dot) && self.is_plain_dir(&dir) {
            self.folders.push((dir, LOCAL));
        }
    }

    /// Whether `path` is a directory and not a symbolic link, as the writer requires; a path that
    /// is missing is not named.
    fn is_plain_dir(&mut self, path: &Path) -> bool {
        match std::fs::symlink_metadata(path) {
            Ok(found) if found.is_dir() => true,
            Ok(_) => {
                self.note(format!(
                    "{} is a symbolic link or not a directory: skipped",
                    shown(path)
                ));
                false
            }
            Err(_) => false,
        }
    }

    fn add_downloaded(&mut self, dir: &Path) -> Result<()> {
        let listed = std::fs::read_dir(dir)
            .with_context(|| format!("cannot read the folder {}", dir.display()))?;
        let mut ids: Vec<_> = listed.filter_map(Result::ok).collect();
        ids.sort_by_key(std::fs::DirEntry::file_name);
        for entry in ids {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if !is_dir || !is_plain_name(&name) {
                self.note(format!(
                    "{} is not a folder of a CI run: skipped",
                    shown(&entry.path())
                ));
                continue;
            }
            let source = self.source_of(&name, &entry.path());
            self.folders.push((entry.path(), source));
            self.add_artifacts(&entry.path(), source);
        }
        Ok(())
    }

    /// The artifacts of a run are the folders `<id>/<name>`, which hold the two files; a run
    /// folder that holds them itself is read too.
    fn add_artifacts(&mut self, run: &Path, source: usize) {
        let Ok(listed) = std::fs::read_dir(run) else {
            return;
        };
        let mut found: Vec<_> = listed.filter_map(Result::ok).collect();
        found.sort_by_key(std::fs::DirEntry::file_name);
        for entry in found {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if is_dir && is_plain_name(&name) {
                self.folders.push((entry.path(), source));
            }
        }
    }

    /// The source of the run `name`, created with the `trusted` of its `source.json`; when the run
    /// is found twice, it is trusted only if both say so.
    fn source_of(&mut self, name: &str, folder: &Path) -> usize {
        let trusted = self.trusted(&folder.join("source.json"), name);
        if let Some(&source) = self.by_name.get(name) {
            self.history.sources[source].trusted &= trusted;
            return source;
        }
        self.history.sources.push(Source {
            label: format!("CI run {name}"),
            trusted,
        });
        let source = self.history.sources.len() - 1;
        self.by_name.insert(name.to_owned(), source);
        source
    }

    /// The `trusted` of `source.json`, written by `botica history fetch` from the answer of the
    /// API; a run with no readable file is not trusted.
    fn trusted(&mut self, path: &Path, name: &str) -> bool {
        let Some(bytes) = self.small_file(path) else {
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

    fn finish(mut self) -> History {
        let folders = std::mem::take(&mut self.folders);
        for (folder, source) in &folders {
            self.read_runs(&folder.join("runs.jsonl"), *source);
        }
        self.settle_runs();
        let named: HashSet<(usize, String)> = self
            .history
            .runs
            .iter()
            .map(|run| (run.source, run.id.clone()))
            .collect();
        let mut seen = HashSet::new();
        for (folder, source) in &folders {
            self.read_findings(&folder.join("findings.jsonl"), *source, &named, &mut seen);
        }
        self.history
    }

    fn read_runs(&mut self, path: &Path, source: usize) {
        let Some(file) = self.open(path) else {
            return;
        };
        let (mut kept, mut skipped) = (Vec::new(), 0);
        each_line(file, |line| match lines::run(source, line) {
            Some(run) if run.started_at >= self.since => kept.push(run),
            Some(_) => {}
            None => skipped += 1,
        });
        self.history.runs.extend(kept);
        self.history.skipped_lines += skipped;
    }

    fn read_findings(
        &mut self,
        path: &Path,
        source: usize,
        named: &HashSet<(usize, String)>,
        seen: &mut HashSet<(usize, String, String, u64)>,
    ) {
        let Some(file) = self.open(path) else {
            return;
        };
        let (mut kept, mut skipped) = (Vec::new(), 0);
        each_line(file, |line| match lines::finding(source, line) {
            Some(finding) => {
                let key = (source, finding.run_id.clone());
                let unique = (
                    source,
                    format!("{}{}", finding.run_id, finding.fingerprint),
                    finding.slug.clone(),
                    finding.occurrence,
                );
                if named.contains(&key) && seen.insert(unique) {
                    kept.push(finding);
                }
            }
            None => skipped += 1,
        });
        self.history.findings.extend(kept);
        self.history.skipped_lines += skipped;
    }

    /// Drops what decision 2 says to ignore: two different run lines that name one run, and a
    /// line repeated in one source, which is read once.
    fn settle_runs(&mut self) {
        let runs = std::mem::take(&mut self.history.runs);
        let mut first: HashMap<(usize, String), [u8; 32]> = HashMap::new();
        let mut clashing: HashSet<(usize, String)> = HashSet::new();
        for run in &runs {
            let key = (run.source, run.id.clone());
            match first.get(&key) {
                Some(raw) if *raw != run.raw => {
                    clashing.insert(key);
                }
                Some(_) => {}
                None => {
                    first.insert(key, run.raw);
                }
            }
        }
        let mut notes = Vec::new();
        for (source, id) in &clashing {
            let label = &self.history.sources[*source].label;
            notes.push(format!(
                "two different run lines name run {id} of the {label}: both are ignored"
            ));
        }
        notes.sort();
        self.history.notes.extend(notes);
        let mut kept = HashSet::new();
        self.history.runs = runs
            .into_iter()
            .filter(|run| {
                let key = (run.source, run.id.clone());
                !clashing.contains(&key) && kept.insert(key)
            })
            .collect();
    }

    /// A regular file within the limits, opened; one past a limit or that is not a regular file
    /// is named in the notes, and one that is missing is not.
    fn open(&mut self, path: &Path) -> Option<File> {
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
        match File::open(path) {
            Ok(file) => {
                self.used += found.len();
                Some(file)
            }
            Err(error) => {
                self.note(format!("cannot read {}: {error}", shown(path)));
                None
            }
        }
    }

    fn small_file(&mut self, path: &Path) -> Option<Vec<u8>> {
        let file = self.open(path)?;
        let mut bytes = Vec::new();
        file.take(FILE_MAX).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    }

    fn note(&mut self, text: String) {
        self.history.notes.push(text);
    }
}

/// Calls `each` on every line of `file` that is not blank, without holding the file in memory.
/// A final line with no `\n` is a line of its own.
fn each_line(file: File, mut each: impl FnMut(&[u8])) {
    let mut reader = BufReader::new(file.take(FILE_MAX));
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if !line.iter().all(u8::is_ascii_whitespace) {
            each(&line);
        }
    }
}

/// A folder name that can stand for a run: at most 256 bytes and no control character.
fn is_plain_name(name: &str) -> bool {
    name.len() <= 256 && !name.chars().any(char::is_control)
}

/// A path for a message, with no control character in it.
fn shown(path: &Path) -> String {
    path.display()
        .to_string()
        .chars()
        .map(|found| if found.is_control() { '?' } else { found })
        .collect()
}
