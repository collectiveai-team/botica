//! Which commit comes before which, for the pairing of a hook run with a CI run (ADR 0019
//! decision 8).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// What the pairing asks of the commits of the clone.
pub trait Ancestry {
    /// Whether `older` is a proper ancestor of `newer`. A commit that the clone does not hold is
    /// no ancestor of anything, so the pairing falls back to the time of the runs.
    fn is_ancestor(&self, older: &str, newer: &str) -> bool;
}

/// The git of the checkout at `root`.
pub struct Git {
    root: PathBuf,
    asked: RefCell<HashMap<(String, String), bool>>,
}

impl Git {
    pub fn at(root: PathBuf) -> Self {
        Self {
            root,
            asked: RefCell::new(HashMap::new()),
        }
    }

    fn ask(&self, older: &str, newer: &str) -> bool {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", older, newer])
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}

impl Ancestry for Git {
    fn is_ancestor(&self, older: &str, newer: &str) -> bool {
        if older == newer {
            return false;
        }
        let key = (older.to_owned(), newer.to_owned());
        if let Some(&known) = self.asked.borrow().get(&key) {
            return known;
        }
        let answer = self.ask(older, newer);
        self.asked.borrow_mut().insert(key, answer);
        answer
    }
}
