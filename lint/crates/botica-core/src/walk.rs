use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Component, Path, PathBuf};

pub struct SourceFile {
    pub path: String,
    pub text: String,
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

/// What `walk_and_lint` collects from every walker thread.
#[derive(Default)]
struct Linted {
    read: usize,
    found: Vec<Diagnostic>,
}

/// Walks `dir` and lints each file on the walker thread that found it, with no list of paths and
/// no barrier between walking and linting. Nobody keeps the texts. Returns the number of files
/// read and the sorted diagnostics.
pub fn walk_and_lint(engine: &Engine, root: &Path, dir: &Path) -> (usize, Vec<Diagnostic>) {
    struct Builder<'a> {
        engine: &'a Engine,
        root: &'a Path,
        into: &'a std::sync::Mutex<Linted>,
    }
    struct Visitor<'a> {
        builder: Builder<'a>,
        buf: Vec<u8>,
        mine: Linted,
    }
    impl<'a> ignore::ParallelVisitorBuilder<'a> for Builder<'a> {
        fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 'a> {
            Box::new(Visitor {
                builder: Builder { ..*self },
                buf: Vec::new(),
                mine: Linted::default(),
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
                        if let Some(text) = read_into(entry.path(), &mut self.buf) {
                            self.mine
                                .found
                                .extend(self.builder.engine.lint_planned(&path, text, &plan));
                            self.mine.read += 1;
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
                all.found.append(&mut self.mine.found);
            }
        }
    }

    let into = std::sync::Mutex::new(Linted::default());
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
    let Linted { read, mut found } = into.into_inner().expect("walk");
    sort(&mut found);
    (read, found)
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

fn read_one(root: &Path, path: &Path) -> Option<SourceFile> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(SourceFile {
        path: relative_path(root, path),
        text,
    })
}

const FIRST_READ: usize = 64 * 1024;

/// A NUL in the first 8 KB marks a binary file, the criterion of git and ripgrep.
const BINARY_PROBE: usize = 8 * 1024;

/// Reads `path` into `buf`, which a worker keeps between files: no allocation per file, and no
/// `statx`/`lseek` for a size hint (most files fit in one read). `None` when the file cannot be
/// read, looks binary (decided from the first read, before reading the rest) or is not UTF-8,
/// like `read_to_string`.
fn read_into<'a>(path: &Path, buf: &'a mut Vec<u8>) -> Option<&'a str> {
    use std::io::{ErrorKind, Read};
    if buf.is_empty() {
        buf.resize(FIRST_READ, 0);
    }
    let mut file = std::fs::File::open(path).ok()?;
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
            Err(_) => return None,
        }
        if !probed && filled >= BINARY_PROBE.min(buf.len()) {
            probed = true;
            if buf[..filled.min(BINARY_PROBE)].contains(&0) {
                return None;
            }
        }
    }
    if !probed && buf[..filled.min(BINARY_PROBE)].contains(&0) {
        return None;
    }
    std::str::from_utf8(&buf[..filled]).ok()
}

/// Like `read_and_lint` for when nobody needs the texts afterwards (no extension): each file's
/// text is dropped as soon as it is linted. Returns the number of files read.
pub fn lint_streaming(engine: &Engine, root: &Path, paths: &[PathBuf]) -> (usize, Vec<Diagnostic>) {
    let found: Vec<Option<Vec<Diagnostic>>> = paths
        .par_iter()
        .map_init(Vec::new, |buf, path| {
            let text = read_into(path, buf)?;
            Some(engine.lint_source(&relative_path(root, path), text))
        })
        .collect();
    let read = found.iter().flatten().count();
    let mut out: Vec<Diagnostic> = found.into_iter().flatten().flatten().collect();
    sort(&mut out);
    (read, out)
}

pub fn read_all(root: &Path, paths: &[PathBuf]) -> Vec<SourceFile> {
    paths
        .par_iter()
        .filter_map(|path| read_one(root, path))
        .collect()
}

/// Reads and lints each file on the same thread, with no barrier between reading and linting.
/// Returns the texts too, for extensions.
pub fn read_and_lint(
    engine: &Engine,
    root: &Path,
    paths: &[PathBuf],
) -> (Vec<SourceFile>, Vec<Diagnostic>) {
    let (sources, found): (Vec<SourceFile>, Vec<Vec<Diagnostic>>) = paths
        .par_iter()
        .filter_map(|path| {
            let file = read_one(root, path)?;
            let found = engine.lint_source(&file.path, &file.text);
            Some((file, found))
        })
        .unzip();
    let mut out: Vec<Diagnostic> = found.into_iter().flatten().collect();
    sort(&mut out);
    (sources, out)
}
