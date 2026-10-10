use super::{relative_path, sort, Walked};
use crate::diagnostic::Diagnostic;
use crate::engine::{language_of, Engine, Plan};
use rayon::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

pub struct SourceFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

impl SourceFile {
    /// The contents as text; `None` when the file is not UTF-8.
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }
}

/// A file the walk found and could not read. Not for being binary, and not for being gone by the
/// time it was opened (a race the run ignores).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: String,
    pub message: String,
}

impl Unreadable {
    /// `None` when the failure is the file having disappeared.
    pub(super) fn of(path: String, error: &io::Error) -> Option<Self> {
        (error.kind() != io::ErrorKind::NotFound).then(|| Self {
            path,
            message: error.to_string(),
        })
    }
}

/// What reading a file the caller named brought.
pub(super) enum Outcome<T> {
    Got(T),
    /// Binary, or gone before it was opened.
    Skipped,
    Failed(Unreadable),
}

impl<T> Outcome<T> {
    fn map<U>(self, change: impl FnOnce(T) -> U) -> Outcome<U> {
        match self {
            Self::Got(value) => Outcome::Got(change(value)),
            Self::Skipped => Outcome::Skipped,
            Self::Failed(failure) => Outcome::Failed(failure),
        }
    }
}

/// A file the caller named: its name as the rules see it, what the engine means to do with it
/// and its bytes. The bytes are empty when the rules go by the name alone and nobody wants the
/// text, and when the file looks binary (`binary`).
pub(super) struct Opened<'a> {
    pub(super) name: String,
    pub(super) plan: Option<Plan>,
    bytes: &'a [u8],
    binary: bool,
}

/// Opens `path` into `buf`. The file is left unread when the rules look only at its name, which
/// also spares a binary file from being dropped. `keep` says the caller holds on to the texts, so
/// a file in a language an extension may ask for is read all the same.
pub(super) fn open_named<'a>(
    engine: &Engine,
    root: &Path,
    path: &Path,
    buf: &'a mut Vec<u8>,
    keep: bool,
) -> Outcome<Opened<'a>> {
    let name = relative_path(root, path);
    let plan = engine.plan(&name);
    let read = match &plan {
        Some(plan) if plan.content => true,
        Some(_) => keep && language_of(&name).is_some(),
        None => keep,
    };
    let opened = |name, plan, bytes, binary| {
        Outcome::Got(Opened {
            name,
            plan,
            bytes,
            binary,
        })
    };
    if !read {
        return opened(name, plan, &[], false);
    }
    match read_into(path, buf) {
        Ok(Some(bytes)) => opened(name, plan, bytes, false),
        Ok(None) => opened(name, plan, &[], true),
        Err(error) => match Unreadable::of(name, &error) {
            Some(failure) => Outcome::Failed(failure),
            None => Outcome::Skipped,
        },
    }
}

/// What the engine finds in a file opened by `open_named`.
pub(super) fn lint_opened(engine: &Engine, opened: &Opened<'_>) -> Vec<Diagnostic> {
    let Some(plan) = &opened.plan else {
        return Vec::new();
    };
    if opened.binary {
        engine.lint_unread(&opened.name, plan)
    } else {
        engine.lint_bytes_planned(&opened.name, opened.bytes, plan)
    }
}

/// The files that were read and the ones that could not be, each in the order given.
fn split<T>(outcomes: Vec<Outcome<T>>) -> (Vec<T>, Vec<Unreadable>) {
    let mut got = Vec::new();
    let mut failed = Vec::new();
    for outcome in outcomes {
        match outcome {
            Outcome::Got(value) => got.push(value),
            Outcome::Skipped => {}
            Outcome::Failed(failure) => failed.push(failure),
        }
    }
    (got, failed)
}

fn walked(found: Vec<Vec<Diagnostic>>, mut unreadable: Vec<Unreadable>) -> Walked {
    let read = found.len();
    let mut diagnostics: Vec<Diagnostic> = found.into_iter().flatten().collect();
    sort(&mut diagnostics);
    unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    Walked {
        read,
        diagnostics,
        unreadable,
        asked_for: Vec::new(),
    }
}

const FIRST_READ: usize = 64 * 1024;

/// A NUL in the first 8 KB marks a binary file, the criterion of git and ripgrep.
const BINARY_PROBE: usize = 8 * 1024;

/// Reads `path` into `buf`, which a worker keeps between files: no allocation per file, and no
/// `statx`/`lseek` for a size hint (most files fit in one read). `Ok(None)` when the file looks
/// binary (decided from the first read, before reading the rest). Whether the bytes are UTF-8 is
/// for the engine to say: a rule that counts bytes still wants a Latin-1 file.
fn read_into<'a>(path: &Path, buf: &'a mut Vec<u8>) -> io::Result<Option<&'a [u8]>> {
    use std::io::{ErrorKind, Read};
    if buf.is_empty() {
        buf.resize(FIRST_READ, 0);
    }
    let mut file = std::fs::File::open(path)?;
    let mut filled = 0;
    let mut probed = false;
    loop {
        if filled == buf.len() {
            buf.resize(buf.len() * 2, 0);
        }
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
        if !probed && filled >= BINARY_PROBE.min(buf.len()) {
            probed = true;
            if buf[..filled.min(BINARY_PROBE)].contains(&0) {
                return Ok(None);
            }
        }
    }
    if !probed && buf[..filled.min(BINARY_PROBE)].contains(&0) {
        return Ok(None);
    }
    Ok(Some(&buf[..filled]))
}

/// Like `read_and_lint` for when nobody needs the texts afterwards (no extension): each file's
/// text is dropped as soon as it is linted.
pub fn lint_streaming(engine: &Engine, root: &Path, paths: &[PathBuf]) -> Walked {
    let outcomes: Vec<Outcome<Vec<Diagnostic>>> = paths
        .par_iter()
        .map_init(Vec::new, |buf, path| {
            open_named(engine, root, path, buf, false).map(|opened| lint_opened(engine, &opened))
        })
        .collect();
    let (found, unreadable) = split(outcomes);
    walked(found, unreadable)
}

/// The files that were read, with their bytes, and the ones that could not be. A file the rules
/// judge by its name alone has no bytes.
pub fn read_all(
    engine: &Engine,
    root: &Path,
    paths: &[PathBuf],
) -> (Vec<SourceFile>, Vec<Unreadable>) {
    let outcomes: Vec<Outcome<SourceFile>> = paths
        .par_iter()
        .map_init(Vec::new, |buf, path| {
            open_named(engine, root, path, buf, true).map(|opened| SourceFile {
                path: opened.name,
                bytes: opened.bytes.to_vec(),
            })
        })
        .collect();
    split(outcomes)
}

/// Reads and lints each file on the same thread, with no barrier between reading and linting.
/// Returns the texts too, for extensions.
pub fn read_and_lint(engine: &Engine, root: &Path, paths: &[PathBuf]) -> (Vec<SourceFile>, Walked) {
    let outcomes: Vec<Outcome<(SourceFile, Vec<Diagnostic>)>> = paths
        .par_iter()
        .map_init(Vec::new, |buf, path| {
            open_named(engine, root, path, buf, true).map(|opened| {
                let found = lint_opened(engine, &opened);
                let file = SourceFile {
                    path: opened.name,
                    bytes: opened.bytes.to_vec(),
                };
                (file, found)
            })
        })
        .collect();
    let (read, unreadable) = split(outcomes);
    let (sources, found): (Vec<SourceFile>, Vec<Vec<Diagnostic>>) = read.into_iter().unzip();
    (sources, walked(found, unreadable))
}
