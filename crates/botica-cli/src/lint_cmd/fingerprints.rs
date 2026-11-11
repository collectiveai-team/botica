//! The fingerprint of each finding in the json and sarif output (ADR 0019 decision 5).

use super::git;
use super::inside::Inside;
use crate::output::eprintln;
use botica_core::fingerprint::FingerprintKey;
use botica_core::report::Format;
use botica_core::walk::SourceFile;
use botica_core::Diagnostic;
use std::collections::HashMap;
use std::path::Path;

/// What the findings were read from.
pub(super) enum Read<'a> {
    /// The texts that the run linted. A path that they lack is read from the index when
    /// `staged`, else from `root`.
    Files {
        sources: &'a [SourceFile],
        root: &'a Path,
        staged: bool,
    },
    /// Commit messages and titles: no finding has a line of a file.
    Messages,
}

/// Sets the fingerprint of each finding when the output carries it or the run records. A
/// `BOTICA_HISTORY_KEY` that is too short leaves every fingerprint out, with a warning.
pub(super) fn stamp(
    format: Format,
    recording: bool,
    inside: &Inside<'_>,
    read: Read<'_>,
    diagnostics: &mut [Diagnostic],
) {
    if !recording && !matches!(format, Format::Json | Format::Sarif) {
        return;
    }
    let key = match FingerprintKey::from_env() {
        Ok(key) => key,
        Err(error) => {
            eprintln!("botica: warning: the findings carry no fingerprint: {error}");
            return;
        }
    };
    match read {
        Read::Messages => {
            for diagnostic in diagnostics {
                diagnostic.fingerprint =
                    Some(key.of_message(&diagnostic.rule, "", &diagnostic.message));
            }
        }
        Read::Files {
            sources,
            root,
            staged,
        } => {
            let linted: HashMap<&str, &SourceFile> = sources
                .iter()
                .map(|source| (source.path.as_str(), source))
                .collect();
            key.stamp(
                diagnostics,
                |path| inside.path(path),
                |path| match linted.get(path) {
                    Some(source) => source.text().map(str::to_owned),
                    None if staged => {
                        git::show("", path).and_then(|bytes| String::from_utf8(bytes).ok())
                    }
                    None => std::fs::read_to_string(root.join(path)).ok(),
                },
            );
        }
    }
}
