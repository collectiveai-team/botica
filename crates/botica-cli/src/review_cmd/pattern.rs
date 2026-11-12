//! A glob of `[[review.areas]]`, read as a `.gitignore` reads it, written as CODEOWNERS lines.

use anyhow::{bail, Result};
use botica_config::expand_braces;

use super::Host;

const ANY_DEPTH: &str = "**";

/// What a glob selects, with every name anchored to the root: a glob with no `/` starts with
/// `**`. `folder` says that the pattern selects what is inside the paths that `segments` name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pattern {
    segments: Vec<String>,
    folder: bool,
}

impl Pattern {
    /// The patterns of a glob of the table, one per alternative of its braces.
    pub(super) fn read(glob: &str) -> Result<Vec<Self>> {
        if glob.contains(['[', '\\']) || glob.contains(char::is_whitespace) {
            bail!(
                "`{glob}` has a character class, an escape or a space, which GitHub and GitLab \
                 read in different ways in a CODEOWNERS file: write the paths without them"
            );
        }
        Ok(expand_braces(glob)
            .iter()
            .map(|alternative| Self::one(alternative))
            .collect())
    }

    fn one(glob: &str) -> Self {
        let folder = glob.ends_with('/');
        let glob = glob.trim_end_matches('/');
        if glob.is_empty() {
            return Self {
                segments: Vec::new(),
                folder: true,
            };
        }
        let anchored = glob.contains('/');
        let mut segments: Vec<String> = Vec::new();
        if !anchored {
            segments.push(ANY_DEPTH.to_owned());
        }
        for part in glob.split('/').filter(|part| !part.is_empty()) {
            let repeated = part == ANY_DEPTH && segments.last().is_some_and(|s| s == ANY_DEPTH);
            if !repeated {
                segments.push(part.to_owned());
            }
        }
        Self { segments, folder }.normalized()
    }

    /// `x/**` and `x/**/*` select what is inside `x`, which `x/` says on both hosts; a folder
    /// that ends in `**` is any folder below, `**/*/`.
    fn normalized(mut self) -> Self {
        let last_two = |s: &Self| -> Option<(String, String)> {
            let n = s.segments.len();
            (n >= 2).then(|| (s.segments[n - 2].clone(), s.segments[n - 1].clone()))
        };
        if self.folder {
            if self.segments.last().is_some_and(|s| s == ANY_DEPTH) {
                self.segments.push("*".to_owned());
            }
            return self;
        }
        if self.segments.last().is_some_and(|s| s == ANY_DEPTH) {
            self.segments.pop();
            self.folder = true;
        } else if last_two(&self).is_some_and(|(a, b)| a == ANY_DEPTH && b == "*") {
            self.segments.truncate(self.segments.len() - 2);
            self.folder = true;
        }
        self
    }

    /// Whether the pattern selects every path of the repository.
    fn everything(&self) -> bool {
        self.segments.iter().all(|s| s == ANY_DEPTH)
    }

    /// The number of names with no wildcard: the more a pattern spells out, the narrower it is.
    pub(super) fn literal_depth(&self) -> usize {
        self.segments
            .iter()
            .filter(|s| !s.contains(['*', '?']))
            .count()
    }

    /// The patterns to write on `host`. A name that is not followed by `/` may be a folder. GitHub
    /// gives a folder its contents unless the last name is `*`; GitLab never does, so the folder
    /// gets a line of its own.
    pub(super) fn lines(&self, host: Host) -> Vec<Self> {
        let mut lines = vec![self.clone()];
        if self.folder || self.everything() {
            return lines;
        }
        let bare_star = self.segments.last().is_some_and(|s| s == "*");
        if host == Host::Gitlab || bare_star {
            lines.push(Self {
                segments: self.segments.clone(),
                folder: true,
            });
        }
        lines
    }

    pub(super) fn is_folder(&self) -> bool {
        self.folder
    }

    /// The text of the pattern without the `/` of a folder.
    pub(super) fn stem(&self) -> String {
        if self.everything() {
            return "*".to_owned();
        }
        let joined = self.segments.join("/");
        if self.segments[0] == ANY_DEPTH {
            joined
        } else {
            format!("/{joined}")
        }
    }

    pub(super) fn text(&self) -> String {
        let stem = self.stem();
        if self.folder && !self.everything() {
            format!("{stem}/")
        } else {
            stem
        }
    }
}
