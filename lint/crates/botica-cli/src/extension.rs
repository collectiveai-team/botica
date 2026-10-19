use std::path::Path;
use std::time::{Duration, Instant};

use botica_core::{walk, Diagnostic, ExtensionRun};
use botica_ext::Selection;

/// How long each step of an extension took, wall time.
#[derive(Debug, Default)]
pub struct ExtensionTime {
    pub start: Duration,
    pub lint: Duration,
    pub shutdown: Duration,
}

impl ExtensionTime {
    pub fn total(&self) -> Duration {
        self.start + self.lint + self.shutdown
    }
}

/// What an extension gave back: what it found, why it stopped if it did not finish cleanly (a
/// failure never takes down what was already found), and how long it took.
pub struct Outcome {
    pub found: Vec<Diagnostic>,
    pub failure: Option<anyhow::Error>,
    pub time: ExtensionTime,
}

/// `--ext`: runs the command, which says which languages it reads, over the texts.
pub fn run(command: &str, sources: &[walk::SourceFile], batch: usize) -> Outcome {
    let argv: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    run_with(
        || ExtensionRun::start_in(&argv, None),
        |run| {
            let files = sources
                .iter()
                .filter_map(|f| Some((f.path.as_str(), f.text()?)));
            run.lint(files, batch)
        },
    )
}

/// A rule of a catalog: runs `argv` from `dir` over the files that the rule applies to, as
/// `(path, text)`. A rule of project scope gets the paths in one `lint/project`, the others the
/// texts in batches of `lint/files`.
pub fn run_rule(process: Process<'_>, files: &[(&str, &str)], how: Send) -> Outcome {
    let Process {
        argv,
        dir,
        selection,
        allowed,
    } = process;
    run_with(
        || ExtensionRun::start_for(argv, Some(dir), selection, allowed),
        |run| match how {
            Send::Files { batch } => run.lint_all(files.iter().copied(), batch),
            Send::Project => run.lint_project(dir, files),
        },
    )
}

/// The process of a rule of a catalog: what runs it and from where, the rule it is asked for with
/// its options, and the rules it may declare.
pub struct Process<'a> {
    pub argv: &'a [String],
    pub dir: &'a Path,
    pub selection: Selection,
    pub allowed: &'a [String],
}

/// How the files reach the process of a rule.
#[derive(Clone, Copy)]
pub enum Send {
    Files { batch: usize },
    Project,
}

fn run_with(
    start: impl FnOnce() -> anyhow::Result<ExtensionRun>,
    lint: impl FnOnce(&mut ExtensionRun) -> anyhow::Result<Vec<Diagnostic>>,
) -> Outcome {
    let mut time = ExtensionTime::default();
    let started = Instant::now();
    let started_run = start();
    time.start = started.elapsed();
    let mut run = match started_run {
        Ok(run) => run,
        Err(error) => {
            return Outcome {
                found: Vec::new(),
                failure: Some(error),
                time,
            }
        }
    };
    let linting = Instant::now();
    let linted = lint(&mut run);
    time.lint = linting.elapsed();
    let found = match linted {
        Ok(found) => found,
        Err(error) => {
            return Outcome {
                found: Vec::new(),
                failure: Some(error),
                time,
            }
        }
    };
    let finishing = Instant::now();
    let failure = run.finish().err();
    time.shutdown = finishing.elapsed();
    Outcome {
        found,
        failure,
        time,
    }
}
