//! What lints the buffers of the server: the engine of the project, behind a trait that a test
//! can fake.

use botica_core::{Buffer, Diagnostic};

/// When the editor shows the findings of a rule: as the text changes (`type` in the `run-on` of
/// the rule), or when the file is saved (`save` without `type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Type,
    Save,
}

/// The work of the rules of project scope: it reads the files on disk and runs on a thread of its
/// own.
pub type ProjectRun = Box<dyn FnOnce() -> Vec<Diagnostic> + Send>;

/// What the server lints the documents with.
pub trait Linter {
    /// A buffer for the file `path`, from the root, holding `text`.
    fn open(&self, path: &str, text: &str) -> Buffer;

    /// The findings of every rule that runs in the editor, in the text of `buffer`.
    fn lint(&self, buffer: &Buffer) -> Vec<Diagnostic>;

    /// When the findings of `rule` are shown; never when `None`.
    fn when(&self, rule: &str) -> Option<When>;

    /// The run of the rules of project scope, if any is active.
    fn project(&self) -> Option<ProjectRun> {
        None
    }

    /// Whether a change of the file `path` changes the configuration of the project.
    fn configures(&self, _path: &str) -> bool {
        false
    }
}

impl<T: Linter + ?Sized> Linter for &T {
    fn open(&self, path: &str, text: &str) -> Buffer {
        (**self).open(path, text)
    }

    fn lint(&self, buffer: &Buffer) -> Vec<Diagnostic> {
        (**self).lint(buffer)
    }

    fn when(&self, rule: &str) -> Option<When> {
        (**self).when(rule)
    }

    fn project(&self) -> Option<ProjectRun> {
        (**self).project()
    }

    fn configures(&self, path: &str) -> bool {
        (**self).configures(path)
    }
}

/// What lints a document: its path from the root of the project, and its text.
pub type Lint<'a> = &'a dyn Fn(&str, &str) -> Vec<Diagnostic>;

/// A function that lints a whole text: its findings show as the text changes.
pub struct Plain<'a>(pub Lint<'a>);

impl Linter for Plain<'_> {
    fn open(&self, path: &str, text: &str) -> Buffer {
        Buffer::new(path, text)
    }

    fn lint(&self, buffer: &Buffer) -> Vec<Diagnostic> {
        (self.0)(buffer.path(), buffer.text())
    }

    fn when(&self, _rule: &str) -> Option<When> {
        Some(When::Type)
    }
}

/// What builds the linter again when the configuration changes, or the reason it cannot.
pub type Reload<'a> = &'a dyn Fn() -> Result<Box<dyn Linter>, String>;
