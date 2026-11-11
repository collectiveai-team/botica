//! The fingerprint of a finding (ADR 0019 decision 5): a keyed hash of its rule, its path and the
//! text around it, with no line number, so that a finding keeps it when the lines above it move.

use crate::diagnostic::Diagnostic;
use std::collections::HashMap;
use std::fmt;

/// The environment variable whose value keys the hash.
pub const KEY_VARIABLE: &str = botica_ext::HISTORY_KEY_VARIABLE;

const CONTEXT: &str = "botica history fingerprint v1";

/// The shortest value of `BOTICA_HISTORY_KEY` that is accepted, in bytes.
pub const MIN_KEY_LEN: usize = 32;

/// A value of `BOTICA_HISTORY_KEY` too short to keep a fingerprint from being guessed offline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortKey {
    pub len: usize,
}

impl fmt::Display for ShortKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{KEY_VARIABLE}` holds {} bytes and needs at least {MIN_KEY_LEN}",
            self.len
        )
    }
}

impl std::error::Error for ShortKey {}

/// The key of the hash, derived from the value of `BOTICA_HISTORY_KEY`.
#[derive(Clone)]
pub struct FingerprintKey {
    key: [u8; 32],
}

impl FingerprintKey {
    /// The key derived from `value`; with no value, or an empty one, from the empty string.
    pub fn new(value: Option<&str>) -> Result<Self, ShortKey> {
        let value = value.unwrap_or("");
        if !value.is_empty() && value.len() < MIN_KEY_LEN {
            return Err(ShortKey { len: value.len() });
        }
        Ok(Self {
            key: blake3::derive_key(CONTEXT, value.as_bytes()),
        })
    }

    /// The key that `BOTICA_HISTORY_KEY` gives this process.
    pub fn from_env() -> Result<Self, ShortKey> {
        Self::new(std::env::var(KEY_VARIABLE).ok().as_deref())
    }

    /// The fingerprint of a finding of `slug` at line `line` (1-based) of `text`: the line and
    /// its nearest non-blank neighbors stand for where it is. A line past the end of the text
    /// counts as a finding about the whole file.
    pub fn of_line(&self, slug: &str, path: &str, text: &str, line: usize) -> String {
        let lines: Vec<&str> = text.lines().collect();
        self.of_line_in(slug, path, &lines, line)
    }

    /// The fingerprint of a finding about a whole file: its three lines are empty.
    pub fn of_file(&self, slug: &str, path: &str) -> String {
        self.hash(&[slug, path, "", "", ""])
    }

    /// The fingerprint of a finding with no line text (about a whole change, or about a message):
    /// its message stands for the three lines.
    pub fn of_message(&self, slug: &str, path: &str, message: &str) -> String {
        self.hash(&[slug, path, &normalized(message)])
    }

    /// Sets the fingerprint of each finding. `project_path` gives the path of a finding from the
    /// root of the project, and `text_of` the text that the run linted at the path of a finding,
    /// which is read once for each path.
    pub fn stamp(
        &self,
        diagnostics: &mut [Diagnostic],
        project_path: impl Fn(&str) -> String,
        mut text_of: impl FnMut(&str) -> Option<String>,
    ) {
        let mut lines_of: HashMap<String, Option<Vec<String>>> = HashMap::new();
        for diagnostic in diagnostics {
            let slug = diagnostic.rule.as_str();
            let fingerprint = if diagnostic.path.is_empty() {
                self.of_message(slug, "", &diagnostic.message)
            } else {
                let path = project_path(&diagnostic.path);
                let lines = (!diagnostic.file_level).then(|| {
                    lines_of.entry(diagnostic.path.clone()).or_insert_with(|| {
                        text_of(&diagnostic.path)
                            .map(|text| text.lines().map(str::to_owned).collect())
                    })
                });
                match lines {
                    Some(Some(lines)) => self.of_line_in(slug, &path, lines, diagnostic.line),
                    _ => self.of_file(slug, &path),
                }
            };
            diagnostic.fingerprint = Some(fingerprint);
        }
    }

    fn of_line_in(&self, slug: &str, path: &str, lines: &[impl AsRef<str>], line: usize) -> String {
        let Some(index) = line.checked_sub(1).filter(|&index| index < lines.len()) else {
            return self.of_file(slug, path);
        };
        let non_blank = |line: &&_| !AsRef::<str>::as_ref(*line).trim().is_empty();
        let above = lines[..index].iter().rev().find(non_blank);
        let below = lines[index + 1..].iter().find(non_blank);
        self.hash(&[
            slug,
            path,
            &normalized(above.map_or("", AsRef::as_ref)),
            &normalized(lines[index].as_ref()),
            &normalized(below.map_or("", AsRef::as_ref)),
        ])
    }

    /// Each field preceded by its length in bytes as a 64-bit little-endian integer; the first
    /// 16 hex digits of the keyed hash.
    fn hash(&self, fields: &[&str]) -> String {
        let mut hasher = blake3::Hasher::new_keyed(&self.key);
        for field in fields {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field.as_bytes());
        }
        hasher.finalize().to_hex()[..16].to_owned()
    }
}

/// Trimmed, with each run of whitespace turned into one space.
fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
