//! What lints the buffers of an editor (`botica server`): the engine of a run, built once for each
//! reading of the configuration, and the rules of project scope at save.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use botica_catalog::RunOn;
use botica_core::{postprocess, Buffer, Diagnostic};
use botica_lsp::{Linter, ProjectRun, When};

use super::{project_files, LintArgs, Setup};
use crate::extension_pool::ExtensionPool;
use crate::project;

/// The files whose change changes the configuration: where it is read from, and the catalogs of
/// the project in `.botica/`.
const CONFIGURATION: [&str; 2] = ["botica.toml", "pyproject.toml"];

/// The rules of the engine (native and ast-grep) for the buffers; the extension rules are the
/// processes of `extensions`.
pub struct Editor {
    setup: Setup,
}

impl Editor {
    pub fn build(project: project::ProjectArgs) -> Result<Self> {
        Ok(Self {
            setup: Setup::build(&LintArgs::for_editor(project))?,
        })
    }

    /// The extension rules of the catalogs of the project.
    pub const fn extensions(&self) -> &ExtensionPool {
        &self.setup.pool
    }
}

/// When the editor shows the findings of a rule that runs on `run_on`: as the text changes with
/// `type`, at save with `save`, never with neither. A rule without `run-on` shows as the text
/// changes.
pub fn when_of(run_on: Option<&[RunOn]>) -> Option<When> {
    match run_on {
        None => Some(When::Type),
        Some(on) if on.contains(&RunOn::Type) => Some(When::Type),
        Some(on) if on.contains(&RunOn::Save) => Some(When::Save),
        Some(_) => None,
    }
}

impl Linter for Editor {
    fn open(&self, path: &str, text: &str) -> Buffer {
        self.setup.engine.open(path, text)
    }

    /// The findings of the engine, with the severities and the ignores of the configuration at
    /// the path, as `botica lint` reports them.
    fn lint(&self, buffer: &Buffer) -> Vec<Diagnostic> {
        postprocess::apply(
            self.setup.engine.lint_buffer(buffer),
            project::verdicts(&self.setup.resolution, self.setup.prefix.as_deref()),
        )
    }

    fn when(&self, rule: &str) -> Option<When> {
        match self.setup.resolution.entry(rule) {
            Some(entry) => when_of(entry.rule.run_on.as_deref()),
            None => Some(When::Type),
        }
    }

    /// The native rules of project scope, as `botica lint` runs them, when one is active.
    fn project(&self) -> Option<ProjectRun> {
        let resolution = Arc::clone(&self.setup.resolution);
        let active = project_files::RULES
            .iter()
            .any(|id| resolution.entry(id).is_some_and(|entry| entry.active));
        if !active {
            return None;
        }
        let root = self.setup.project_root.clone();
        let prefix = self.setup.prefix.clone();
        Some(Box::new(move || {
            let found = project_files::findings(&resolution, &root, prefix.as_deref());
            postprocess::apply(found, project::verdicts(&resolution, prefix.as_deref()))
        }))
    }

    fn configures(&self, path: &str) -> bool {
        let path = Path::new(path);
        path.file_name()
            .is_some_and(|name| CONFIGURATION.iter().any(|known| name == *known))
            || path.components().any(|part| part.as_os_str() == ".botica")
    }
}
