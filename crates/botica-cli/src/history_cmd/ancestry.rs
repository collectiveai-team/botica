//! Which commit comes before which, for the pairing of a hook run with a CI run (ADR 0019
//! decision 8).

use std::collections::HashSet;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// What the pairing asks of the commits of the clone.
pub trait Ancestry {
    /// The commits of `among` that are proper ancestors of `head`. `None` when the clone does not
    /// hold `head`, which sends the pairing to the time of the runs.
    fn ancestors(&self, head: &str, among: &HashSet<&str>) -> Option<HashSet<String>>;
}

/// The git of the checkout at `root`: one `git rev-list` for each head, read as a stream.
pub struct Git {
    root: PathBuf,
}

impl Git {
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }
}

impl Ancestry for Git {
    fn ancestors(&self, head: &str, among: &HashSet<&str>) -> Option<HashSet<String>> {
        let mut child = Command::new("git")
            .args(["rev-list", "--end-of-options", head])
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let mut found = HashSet::new();
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if line != head && among.contains(line.as_str()) {
                    found.insert(line);
                }
            }
        }
        child
            .wait()
            .is_ok_and(|status| status.success())
            .then_some(found)
    }
}
