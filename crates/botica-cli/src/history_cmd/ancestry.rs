//! Which commit comes before which, for the pairing of a hook run with a CI run (ADR 0019
//! decision 8). The commits of the window are read once, as a graph of parents.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// How many commits of the window the graph holds; a clone with more leaves the older ones out,
/// and the pairing falls back to the time of the runs for them.
pub const MAX_COMMITS: usize = 200_000;

/// The parents of the commits that the clone holds, from the heads that it holds.
#[derive(Debug, Default)]
pub struct Commits {
    parents: HashMap<String, Vec<String>>,
    heads: HashSet<String>,
}

impl Commits {
    pub fn new(heads: HashSet<String>, parents: HashMap<String, Vec<String>>) -> Self {
        Self { parents, heads }
    }

    /// Whether the clone holds `head`, so that its ancestry is known.
    pub fn knows(&self, head: &str) -> bool {
        self.heads.contains(head)
    }

    /// The commits of `among` that are proper ancestors of `head`.
    pub fn ancestors(&self, head: &str, among: &HashSet<&str>) -> HashSet<String> {
        let mut found = HashSet::new();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = vec![head];
        while let Some(commit) = stack.pop() {
            if !seen.insert(commit) {
                continue;
            }
            if commit != head && among.contains(commit) {
                found.insert(commit.to_owned());
            }
            if let Some(parents) = self.parents.get(commit) {
                stack.extend(parents.iter().map(String::as_str));
            }
        }
        found
    }
}

/// What the pairing asks of the clone.
pub trait Ancestry {
    /// The commits that were made since `since` (Unix seconds) on the way to each of `heads`, with
    /// their parents; only the heads that the clone holds are known.
    fn commits(&self, heads: &[&str], since: u64) -> Commits;
}

/// The git of the checkout at `root`: one `git cat-file` to find the heads that the clone holds
/// and one `git log` over all of them, whatever the number of runs.
pub struct Git {
    root: PathBuf,
}

impl Git {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        command
    }

    /// The heads that are commits of the clone.
    fn held(&self, heads: &[&str]) -> Vec<String> {
        let Ok(mut child) = self.command(&["cat-file", "--batch-check"]).spawn() else {
            return Vec::new();
        };
        if let Some(mut stdin) = child.stdin.take() {
            let wanted: String = heads.iter().map(|head| format!("{head}\n")).collect();
            if stdin.write_all(wanted.as_bytes()).is_err() {
                return Vec::new();
            }
        }
        let mut held = Vec::new();
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let mut words = line.split(' ');
                if let (Some(name), Some("commit")) = (words.next(), words.next()) {
                    held.push(name.to_owned());
                }
            }
        }
        let _ = child.wait();
        held
    }
}

impl Ancestry for Git {
    fn commits(&self, heads: &[&str], since: u64) -> Commits {
        let held = self.held(heads);
        if held.is_empty() {
            return Commits::default();
        }
        let since = format!("--since={}", since.saturating_sub(7 * 24 * 60 * 60));
        let limit = format!("--max-count={MAX_COMMITS}");
        let mut args = vec!["log", "--format=%H %P", &since, &limit, "--end-of-options"];
        args.extend(held.iter().map(String::as_str));
        let Ok(mut child) = self.command(&args).spawn() else {
            return Commits::default();
        };
        drop(child.stdin.take());
        let mut parents = HashMap::new();
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let mut words = line.split(' ');
                if let Some(commit) = words.next() {
                    parents.insert(commit.to_owned(), words.map(str::to_owned).collect());
                }
            }
        }
        let _ = child.wait();
        Commits::new(held.into_iter().collect(), parents)
    }
}
