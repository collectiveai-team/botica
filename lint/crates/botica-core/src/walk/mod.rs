mod read;

use crate::diagnostic::Diagnostic;
use crate::engine::Engine;
use ignore::WalkBuilder;
use std::path::{Component, Path, PathBuf};

use read::{lint_opened, open_named, Outcome};
pub use read::{lint_streaming, read_all, read_and_lint, SourceFile, Unreadable};

/// Directories of tools and caches that hold no code of the project. A walk does not enter them
/// unless it is told to start there or `extend-include` asks for something inside. The other
/// hidden directories (`.github`, `.agents`…) are walked.
const TOOL_DIRS: [&str; 24] = [
    ".git",
    ".hg",
    ".svn",
    ".bzr",
    ".direnv",
    ".eggs",
    ".git-rewrite",
    ".ipynb_checkpoints",
    ".mypy_cache",
    ".nox",
    ".pants.d",
    ".pyenv",
    ".pytest_cache",
    ".pytype",
    ".ruff_cache",
    ".tox",
    ".venv",
    ".vscode",
    ".idea",
    ".cache",
    ".botica",
    ".next",
    ".gradle",
    ".terraform",
];

/// One walk of the tree under `dir`, honouring `.gitignore` and the list of directories of tools,
/// on several threads; sorted, so the result does not depend on thread timing. Keeps the files
/// some rule of the engine applies to. Paths are matched relative to `root`.
pub fn discover(dir: &Path, root: &Path, engine: &Engine) -> Discovered {
    let found = std::sync::Mutex::new(Discovered::default());
    WalkBuilder::new(dir)
        .hidden(false)
        .build_parallel()
        .run(|| {
            let mut mine = Flush {
                found: Discovered::default(),
                into: &found,
            };
            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        if pruned(engine, root, &entry) {
                            return ignore::WalkState::Skip;
                        }
                        if entry.file_type().is_some_and(|t| t.is_file()) {
                            let path = relative_path(root, entry.path());
                            if engine.plan(&path).is_some() {
                                if engine.is_asked_for(&path) {
                                    mine.found.asked_for.push(path);
                                }
                                mine.found.paths.push(entry.into_path());
                            }
                        }
                    }
                    Err(error) => mine.found.unreadable.extend(failure_of(root, &error)),
                }
                ignore::WalkState::Continue
            })
        });
    let mut found = found.into_inner().expect("walk");
    found.paths.sort_unstable();
    found.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// What a walk that only lists brings back.
#[derive(Debug, Default)]
pub struct Discovered {
    /// The files some rule applies to, sorted.
    pub paths: Vec<PathBuf>,
    /// Places the walk could not list (a directory nobody may read), sorted by path.
    pub unreadable: Vec<Unreadable>,
    /// Of those files, the ones `extend-include` asks for, so `walk_asked_for` skips them.
    pub asked_for: Vec<String>,
}

/// The files `extend-include` asks for that the walk of `dir` did not reach on its own, because
/// `.gitignore` or the list of directories of tools left them out (`reached` are the ones it did
/// reach). A walk that ignores those two, but only enters the directories where something asked
/// for may be, and still honours `extend-exclude`. Sorted.
pub fn walk_asked_for(dir: &Path, root: &Path, engine: &Engine, reached: &[String]) -> Discovered {
    let reached: std::collections::HashSet<&str> = reached.iter().map(String::as_str).collect();
    let found = std::sync::Mutex::new(Discovered::default());
    WalkBuilder::new(dir)
        .standard_filters(false)
        .build_parallel()
        .run(|| {
            let mut mine = Flush {
                found: Discovered::default(),
                into: &found,
            };
            let reached = &reached;
            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        if !worth_entering(engine, root, &entry) {
                            return ignore::WalkState::Skip;
                        }
                        let path = relative_path(root, entry.path());
                        if entry.file_type().is_some_and(|t| t.is_file())
                            && engine.is_asked_for(&path)
                            && !reached.contains(path.as_str())
                            && engine.plan(&path).is_some()
                        {
                            mine.found.paths.push(entry.into_path());
                        }
                    }
                    Err(error) => mine.found.unreadable.extend(failure_of(root, &error)),
                }
                ignore::WalkState::Continue
            })
        });
    let mut found = found.into_inner().expect("walk");
    found.paths.sort_unstable();
    found.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// Whether the walk for what `extend-include` asks for goes on into this entry: not `.git`, not
/// what `extend-exclude` leaves out, and for a directory only if something asked for may be in it.
fn worth_entering(engine: &Engine, root: &Path, entry: &ignore::DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
    let path = relative_path(root, entry.path());
    entry.file_name() != ".git"
        && !(engine.has_exclusion() && engine.skips_entry(&path, is_dir))
        && (!is_dir || engine.may_hold_what_is_asked_for(&path))
}

/// A failure of the walk itself, such as a directory it could not list, as a path it could not
/// read. Problems with the ignore files are not failures to read the tree.
fn failure_of(root: &Path, error: &ignore::Error) -> Option<Unreadable> {
    match error {
        ignore::Error::WithDepth { err, .. } => failure_of(root, err),
        ignore::Error::WithPath { path, err } => {
            Unreadable::of(relative_path(root, path), err.io_error()?)
        }
        _ => None,
    }
}

/// What a walk that also lints brings back.
#[derive(Debug, Default)]
pub struct Walked {
    /// Files linted: the ones read, and the ones judged by their name alone, which includes the
    /// binary ones. Not the unreadable ones.
    pub read: usize,
    /// Sorted by file, position and rule, whatever the threads did.
    pub diagnostics: Vec<Diagnostic>,
    /// Sorted by path.
    pub unreadable: Vec<Unreadable>,
    /// Of the files linted, the ones `extend-include` asks for, so `walk_asked_for` skips them.
    pub asked_for: Vec<String>,
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
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    let failure = failure_of(self.builder.root, &error);
                    self.mine.unreadable.extend(failure);
                    return ignore::WalkState::Continue;
                }
            };
            if pruned(self.builder.engine, self.builder.root, &entry) {
                return ignore::WalkState::Skip;
            }
            if entry.file_type().is_some_and(|t| t.is_file()) {
                self.lint_file(entry.path());
            }
            ignore::WalkState::Continue
        }
    }
    impl Visitor<'_> {
        /// Reads the file if some rule needs its content and lints it; counts it unless nothing
        /// applies to it.
        fn lint_file(&mut self, path: &Path) {
            let Visitor { builder, buf, mine } = self;
            match open_named(builder.engine, builder.root, path, buf, false) {
                Outcome::Got(opened) if opened.plan.is_some() => {
                    if builder.engine.is_asked_for(&opened.name) {
                        mine.asked_for.push(opened.name.clone());
                    }
                    mine.diagnostics
                        .extend(lint_opened(builder.engine, &opened));
                    mine.read += 1;
                }
                Outcome::Failed(failure) => mine.unreadable.push(failure),
                Outcome::Got(_) | Outcome::Skipped => {}
            }
        }
    }
    impl Drop for Visitor<'_> {
        fn drop(&mut self) {
            if let Ok(mut all) = self.builder.into.lock() {
                all.read += self.mine.read;
                all.diagnostics.append(&mut self.mine.diagnostics);
                all.unreadable.append(&mut self.mine.unreadable);
                all.asked_for.append(&mut self.mine.asked_for);
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
        .hidden(false)
        .threads(rayon::current_num_threads())
        .build_parallel()
        .visit(&mut builder);
    let mut walked = into.into_inner().expect("walk");
    sort(&mut walked.diagnostics);
    walked.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    walked
}

/// The order of the output: by file, then position, then rule, whatever the threads did.
pub(super) fn sort(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
}

/// Whether `name` is in `TOOL_DIRS`. All of them start with a dot, which spares nearly every
/// entry the comparison with the list.
fn is_tool_dir(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.') && TOOL_DIRS.iter().any(|tool| name == *tool)
}

/// Whether the walk leaves this entry out: a directory of a tool, or what the exclusion names.
/// The directory a walk starts from is never pruned: naming it is asking for it (`--force-exclude`
/// decides that before the walk).
fn pruned(engine: &Engine, root: &Path, entry: &ignore::DirEntry) -> bool {
    entry.depth() > 0
        && (is_tool_dir(entry.file_name())
            || (engine.has_exclusion()
                && engine.skips_entry(
                    &relative_path(root, entry.path()),
                    entry.file_type().is_some_and(|t| t.is_dir()),
                )))
}

/// A walker thread's findings, handed over to the shared ones when the thread's visitor drops.
struct Flush<'a> {
    found: Discovered,
    into: &'a std::sync::Mutex<Discovered>,
}

impl Drop for Flush<'_> {
    fn drop(&mut self) {
        if let Ok(mut all) = self.into.lock() {
            all.paths.append(&mut self.found.paths);
            all.unreadable.append(&mut self.found.unreadable);
            all.asked_for.append(&mut self.found.asked_for);
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
