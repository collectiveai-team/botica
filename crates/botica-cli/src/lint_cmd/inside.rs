//! Which paths of findings are inside the project (ADR 0019 decisions 4, 5 and 7): only those get
//! a path in the history and a fingerprint of their lines.

use super::baseline::from_root;
use std::path::Path;

/// The paths of findings of one run, seen from the directory `prefix` inside the project at
/// `root`.
pub(super) struct Inside<'a> {
    pub(super) root: &'a Path,
    pub(super) prefix: Option<&'a str>,
}

impl Inside<'_> {
    /// The path of a finding from the root of the project, with `/` separators; `None` when it is
    /// empty, absolute, a URI, leaves the root as written, or names a file that a symbolic link
    /// takes out of the root.
    pub(super) fn path(&self, path: &str) -> Option<String> {
        let path = path.replace('\\', "/");
        let first = path.split('/').next().unwrap_or_default();
        if path.is_empty() || path.starts_with('/') || first.contains(':') {
            return None;
        }
        let mut depth = self.prefix.map_or(0, |prefix| prefix.split('/').count());
        for part in path.split('/') {
            match part {
                "." | "" => {}
                ".." if depth == 0 => return None,
                ".." => depth -= 1,
                _ => depth += 1,
            }
        }
        let at_root = from_root(self.prefix, &path);
        if at_root.is_empty() || !self.stays_inside(&at_root) {
            return None;
        }
        Some(at_root)
    }

    /// Whether the file at `at_root` resolves inside the root; a file that does not exist on disk
    /// (a staged one, a deleted one) is judged by its path alone.
    fn stays_inside(&self, at_root: &str) -> bool {
        let Ok(root) = std::fs::canonicalize(self.root) else {
            return true;
        };
        match std::fs::canonicalize(root.join(at_root)) {
            Ok(resolved) => resolved.starts_with(&root),
            Err(_) => true,
        }
    }
}
