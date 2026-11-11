//! What the tests of `botica history report` share: a history written by hand, and the report.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{command, project_in};

pub const HASH: &str = "0123456789abcdef";
pub const OTHER_HASH: &str = "fedcba9876543210";

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs()
}

/// The 16 hex digits of a small number, for run ids and fingerprints.
pub fn id(number: u64) -> String {
    format!("{number:016x}")
}

/// An empty project with a `.git`, where the history is written by hand.
pub fn dir(name: &str) -> PathBuf {
    project_in("history-report", name, None)
}

/// A run line. `rules` is a list of `(slug, hits, suppressed, baselined, hash)`.
pub struct RunSpec {
    pub id: u64,
    pub ago: u64,
    pub origin: &'static str,
    pub mode: &'static str,
    pub branch: Option<&'static str>,
    pub commit: Option<String>,
    pub head: Option<String>,
    pub rules: Vec<(&'static str, u64, u64, u64, Option<&'static str>)>,
}

impl RunSpec {
    pub fn new(id: u64, origin: &'static str, mode: &'static str) -> Self {
        Self {
            id,
            ago: 3600,
            origin,
            mode,
            branch: Some("topic"),
            commit: None,
            head: None,
            rules: Vec::new(),
        }
    }

    pub fn ago(mut self, seconds: u64) -> Self {
        self.ago = seconds;
        self
    }

    pub fn branch(mut self, branch: Option<&'static str>) -> Self {
        self.branch = branch;
        self
    }

    pub fn commit(mut self, commit: &str) -> Self {
        self.commit = Some(commit.to_owned());
        self
    }

    pub fn head(mut self, head: &str) -> Self {
        self.head = Some(head.to_owned());
        self
    }

    pub fn rule(mut self, slug: &'static str, hits: u64, hash: Option<&'static str>) -> Self {
        self.rules.push((slug, hits, 0, 0, hash));
        self
    }

    pub fn counted(
        mut self,
        slug: &'static str,
        counts: (u64, u64, u64),
        hash: Option<&'static str>,
    ) -> Self {
        self.rules.push((slug, counts.0, counts.1, counts.2, hash));
        self
    }

    pub fn line(&self) -> Value {
        let rules: serde_json::Map<String, Value> = self
            .rules
            .iter()
            .map(|(slug, hits, suppressed, baselined, hash)| {
                (
                    (*slug).to_owned(),
                    json!({
                        "hits": hits,
                        "suppressed": suppressed,
                        "baselined": baselined,
                        "hash": hash,
                    }),
                )
            })
            .collect();
        json!({
            "v": 1,
            "run_id": id(self.id),
            "started_at": now() - self.ago,
            "commit": self.commit,
            "head": self.head,
            "branch": self.branch,
            "botica": "0.20.2",
            "rules_fingerprint": "0".repeat(64),
            "key_id": null,
            "mode": self.mode,
            "origin": self.origin,
            "rules": rules,
        })
    }
}

/// A finding line of run `run` for `slug`, whose fingerprint is `fingerprint`.
pub fn finding(run: u64, slug: &str, fingerprint: u64) -> Value {
    json!({
        "v": 1,
        "run_id": id(run),
        "code": "BT002",
        "slug": slug,
        "severity": "warning",
        "path": "a.py",
        "line": 2,
        "fingerprint": id(fingerprint),
        "occurrence": 0,
    })
}

fn join(lines: &[Value]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// Writes `runs.jsonl` and `findings.jsonl` into `folder`.
pub fn write_in(folder: &Path, runs: &[RunSpec], findings: &[Value]) {
    fs::create_dir_all(folder).expect("create the folder");
    let lines: Vec<Value> = runs.iter().map(RunSpec::line).collect();
    fs::write(folder.join("runs.jsonl"), join(&lines)).expect("write the runs");
    fs::write(folder.join("findings.jsonl"), join(findings)).expect("write the findings");
}

pub fn history_dir(dir: &Path) -> PathBuf {
    dir.join(".botica").join("history")
}

/// The local history of `dir`.
pub fn write_local(dir: &Path, runs: &[RunSpec], findings: &[Value]) {
    write_in(&dir.join(".botica").join("history"), runs, findings);
}

/// A downloaded run `name` under `downloads`, with the `source.json` that says whether it is
/// trusted.
pub fn write_downloaded(
    downloads: &Path,
    name: &str,
    trusted: bool,
    runs: &[RunSpec],
    findings: &[Value],
) {
    let folder = downloads.join(name);
    write_in(&folder, runs, findings);
    fs::write(
        folder.join("source.json"),
        json!({ "id": name, "trusted": trusted }).to_string(),
    )
    .expect("write source.json");
}

/// `botica history report` in `dir`, outside any CI.
pub fn report(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut all = vec!["history", "report"];
    all.extend_from_slice(args);
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_botica"));
    command
        .args(all)
        .current_dir(dir)
        .env_remove("GITHUB_ACTIONS");
    command.output().expect("run botica")
}

pub fn help(dir: &Path) -> std::process::Output {
    command(dir, &["history", "report", "--help"])
}

const SECTIONS: [&str; 7] = [
    "Runs read",
    "Caught before the commit",
    "Reached CI",
    "Rules that are not comparable",
    "Rules with no findings",
    "Rules with many suppressions",
    "Findings of the roles that refute did not confirm",
];

/// The text of one section of the report, from its title to the next title, in either format.
pub fn section(text: &str, title: &str) -> String {
    let heading = |line: &str| SECTIONS.contains(&line.trim_start_matches("## "));
    text.lines()
        .skip_while(|line| line.trim_start_matches("## ") != title)
        .skip(1)
        .take_while(|line| !heading(line))
        .collect::<Vec<_>>()
        .join("\n")
}
