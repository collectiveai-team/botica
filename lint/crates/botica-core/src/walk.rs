use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io;
use std::path::{Component, Path, PathBuf};

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

/// One walk of the tree under `dir`, honouring `.gitignore` and skipping hidden files, on several
/// threads; sorted, so the result does not depend on thread timing. Keeps the files some rule of
/// the engine applies to. Paths are matched relative to `root`.
pub fn discover(dir: &Path, root: &Path, engine: &Engine) -> Vec<PathBuf> {
    let found = std::sync::Mutex::new(Vec::new());
    WalkBuilder::new(dir).hidden(true).build_parallel().run(|| {
        let mut mine = Flush {
            paths: Vec::new(),
            into: &found,
        };
        Box::new(move |entry| {
            if let Ok(entry) = entry {
                if pruned(engine, root, &entry) {
                    return ignore::WalkState::Skip;
                }
                if entry.file_type().is_some_and(|t| t.is_file())
                    && engine.plan(&relative_path(root, entry.path())).is_some()
                {
                    mine.paths.push(entry.into_path());
                }
            }
            ignore::WalkState::Continue
        })
    });
    let mut paths = found.into_inner().expect("walk");
    paths.sort_unstable();
    paths
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
    fn of(path: String, error: &io::Error) -> Option<Self> {
        (error.kind() != io::ErrorKind::NotFound).then(|| Self {
            path,
            message: error.to_string(),
        })
    }
}

/// What a walk that also lints brings back.
#[derive(Debug, Default)]
pub struct Walked {
    /// Files read and linted; not the binary ones or the unreadable ones.
    pub read: usize,
    /// Sorted by file, position and rule, whatever the threads did.
    pub diagnostics: Vec<Diagnostic>,
    /// Sorted by path.
    pub unreadable: Vec<Unreadable>,
}

/// Walks `dir` and lints each file on the walker thread that found it, with no list of paths and
/// no barrier between walking and linting. Nobody keeps the texts.
pub fn walk_and_lint(engine: &Engine, root: &Path, dir: &Path) -> Walked {
    struct Builder<'a> {
        engine: &'a Engine,
        root: &'a Path,
        into: &'a std::sync::Mutex<Walked>,
    }
    struct Visitor<'a> {
        builder: Builder<'a>,
        buf: Vec<u8>,
        mine: Walked,
    }
    impl<'a> ignore::ParallelVisitorBuilder<'a> for Builder<'a> {
        fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 'a> {
            Box::new(Visitor {
                builder: Builder { ..*self },
                buf: Vec::new(),
                mine: Walked::default(),
            })
        }
    }
    impl ignore::ParallelVisitor for Visitor<'_> {
        fn visit(&mut self, entry: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
            if let Ok(entry) = entry {
                if pruned(self.builder.engine, self.builder.root, &entry) {
                    return ignore::WalkState::Skip;
                }
                if entry.file_type().is_some_and(|t| t.is_file()) {
                    let path = relative_path(self.builder.root, entry.path());
                    if let Some(plan) = self.builder.engine.plan(&path) {
                        match read_into(entry.path(), &mut self.buf) {
                            Ok(Some(bytes)) => {
                                self.mine.diagnostics.extend(
                                    self.builder.engine.lint_bytes_planned(&path, bytes, &plan),
                                );
                                self.mine.read += 1;
                            }
                            Ok(None) => {}
                            Err(error) => self.mine.unreadable.extend(Unreadable::of(path, &error)),
                        }
                    }
                }
            }
            ignore::WalkState::Continue
        }
    }
    impl Drop for Visitor<'_> {
        fn drop(&mut self) {
            if let Ok(mut all) = self.builder.into.lock() {
                all.read += self.mine.read;
                all.diagnostics.append(&mut self.mine.diagnostics);
                all.unreadable.append(&mut self.mine.unreadable);
            }
        }
    }

    let into = std::sync::Mutex::new(Walked::default());
    let mut builder = Builder {
        engine,
        root,
        into: &into,
    };
    WalkBuilder::new(dir)
        .hidden(true)
        .threads(rayon::current_num_threads())
        .build_parallel()
        .visit(&mut builder);
    let mut walked = into.into_inner().expect("walk");
    sort(&mut walked.diagnostics);
    walked.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    walked
}

/// The order of the output: by file, then position, then rule, whatever the threads did.
fn sort(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
}

/// Whether the exclusion leaves this entry out. The directory a walk starts from is never
/// pruned: naming it is asking for it (`--force-exclude` decides that before the walk).
fn pruned(engine: &Engine, root: &Path, entry: &ignore::DirEntry) -> bool {
    engine.has_exclusion()
        && entry.depth() > 0
        && engine.skips_entry(
            &relative_path(root, entry.path()),
            entry.file_type().is_some_and(|t| t.is_dir()),
        )
}

/// A walker thread's paths, handed over to the shared list when the thread's visitor drops.
struct Flush<'a> {
    paths: Vec<PathBuf>,
    into: &'a std::sync::Mutex<Vec<PathBuf>>,
}

impl Drop for Flush<'_> {
    fn drop(&mut self) {
        if let Ok(mut all) = self.into.lock() {
            all.append(&mut self.paths);
        }
    }
}

/// `path` as the rules and the output see it: from `root` and without `./`.
pub fn relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative: PathBuf = relative
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    relative.to_string_lossy().into_owned()
}

/// What reading a file the caller named brought.
enum Outcome<T> {
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

/// Reads `path` into `buf`: its name as the rules see it, and its bytes.
fn read_named<'a>(root: &Path, path: &Path, buf: &'a mut Vec<u8>) -> Outcome<(String, &'a [u8])> {
    match read_into(path, buf) {
        Ok(Some(bytes)) => Outcome::Got((relative_path(root, path), bytes)),
        Ok(None) => Outcome::Skipped,
        Err(error) => match Unreadable::of(relative_path(root, path), &error) {
            Some(failure) => Outcome::Failed(failure),
            None => Outcome::Skipped,
        },
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
            read_named(root, path, buf).map(|(name, bytes)| engine.lint_bytes(&name, bytes))
        })
        .collect();
    let (found, unreadable) = split(outcomes);
    walked(found, unreadable)
}

/// The files that were read, with their bytes, and the ones that could not be.
pub fn read_all(root: &Path, paths: &[PathBuf]) -> (Vec<SourceFile>, Vec<Unreadable>) {
    let outcomes: Vec<Outcome<SourceFile>> = paths
        .par_iter()
        .map_init(Vec::new, |buf, path| {
            read_named(root, path, buf).map(|(path, bytes)| SourceFile {
                path,
                bytes: bytes.to_vec(),
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
            read_named(root, path, buf).map(|(path, bytes)| {
                let found = engine.lint_bytes(&path, bytes);
                let file = SourceFile {
                    path,
                    bytes: bytes.to_vec(),
                };
                (file, found)
            })
        })
        .collect();
    let (read, unreadable) = split(outcomes);
    let (sources, found): (Vec<SourceFile>, Vec<Vec<Diagnostic>>) = read.into_iter().unzip();
    (sources, walked(found, unreadable))
}
