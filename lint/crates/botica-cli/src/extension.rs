use std::time::{Duration, Instant};

use botica_core::{walk, Diagnostic, ExtensionRun};

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

pub fn run(command: &str, sources: &[walk::SourceFile], batch: usize) -> Outcome {
    let mut time = ExtensionTime::default();
    let argv: Vec<String> = command.split_whitespace().map(str::to_owned).collect();
    let started = Instant::now();
    let started_run = ExtensionRun::start(&argv);
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
    let files = sources.iter().map(|f| (f.path.as_str(), f.text.as_str()));
    let linting = Instant::now();
    let linted = run.lint(files, batch);
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
