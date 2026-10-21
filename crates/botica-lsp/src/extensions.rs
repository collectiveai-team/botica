//! The extension rules in the server: what the server needs of them, and the thread that serves
//! one rule.

use std::io;
use std::sync::mpsc::{Receiver, Sender};

use botica_core::Diagnostic;
use serde_json::Value;

use crate::linter::When;

/// How many times the server starts the process of a rule again after it failed.
pub const MAX_RESTARTS: usize = 3;

/// The extension rules of the catalogs of the project, as the server reaches them. A rule is named by a key.
pub trait Extensions: Sync {
    /// The keys of the rules that read the file at `path` and whose findings show at `when`, none
    /// when no rule does.
    fn at(&self, path: &str, when: When) -> Vec<String>;

    /// Starts the process of the rule `key`.
    ///
    /// # Errors
    /// The reason the process did not start.
    fn start(&self, key: &str) -> Result<Box<dyn Process + '_>, String>;
}

/// The running process of one rule.
pub trait Process {
    /// The findings of the rule in `text`, the content of the file `path`.
    ///
    /// # Errors
    /// The reason the process failed; it is not used again.
    fn lint(&mut self, path: &str, text: &str) -> Result<Vec<Diagnostic>, String>;

    /// Ends the process.
    fn finish(self: Box<Self>);
}

/// What the loop of the server waits for.
pub enum Event {
    Input(io::Result<Option<Value>>),
    Found {
        key: String,
        uri: String,
        revision: u64,
        found: Vec<Diagnostic>,
    },
    /// A message for the editor about a rule that failed.
    Warn(String),
    /// What the rules of project scope found in the run `generation`.
    Project {
        generation: u64,
        found: Vec<Diagnostic>,
    },
}

/// A document for a rule to lint, as it was at `revision`.
pub struct Job {
    pub uri: String,
    pub revision: u64,
    pub path: String,
    pub text: String,
}

/// Serves the rule `key` until `jobs` is closed: starts its process when the first job comes and
/// ends it at the end. A process that fails is started again, up to `MAX_RESTARTS` times in a
/// row, and the editor is told each time.
pub fn work<'a>(
    key: &str,
    extensions: &'a dyn Extensions,
    jobs: &Receiver<Job>,
    events: &Sender<Event>,
) {
    let mut process: Option<Box<dyn Process + 'a>> = None;
    let mut failures = 0;
    while let Ok(first) = jobs.recv() {
        for job in latest_of_each_document(first, jobs) {
            while failures <= MAX_RESTARTS {
                match attempt(&mut process, key, extensions, &job) {
                    Ok(found) => {
                        failures = 0;
                        let _ = events.send(Event::Found {
                            key: key.to_owned(),
                            uri: job.uri.clone(),
                            revision: job.revision,
                            found,
                        });
                        break;
                    }
                    Err(reason) => {
                        process = None;
                        failures += 1;
                        let _ = events.send(Event::Warn(failure_message(key, &reason, failures)));
                    }
                }
            }
        }
    }
    if let Some(running) = process {
        running.finish();
    }
}

fn attempt<'a>(
    process: &mut Option<Box<dyn Process + 'a>>,
    key: &str,
    extensions: &'a dyn Extensions,
    job: &Job,
) -> Result<Vec<Diagnostic>, String> {
    let running = match process.take() {
        Some(running) => running,
        None => extensions.start(key)?,
    };
    process.insert(running).lint(&job.path, &job.text)
}

fn failure_message(key: &str, reason: &str, failures: usize) -> String {
    let next = if failures > MAX_RESTARTS {
        "it will not run again until the server restarts"
    } else {
        "starting it again"
    };
    format!("botica: the extension rule `{key}` failed ({reason}); {next}")
}

/// The jobs that wait, the last one of each document: an older text is not worth linting.
fn latest_of_each_document(first: Job, jobs: &Receiver<Job>) -> Vec<Job> {
    let mut waiting = vec![first];
    waiting.extend(jobs.try_iter());
    let mut kept: Vec<Job> = Vec::new();
    for job in waiting.into_iter().rev() {
        if kept.iter().all(|other| other.uri != job.uri) {
            kept.push(job);
        }
    }
    kept.reverse();
    kept
}
