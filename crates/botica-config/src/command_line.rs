use botica_catalog::Catalog;

use crate::issue::{issue, ConfigError};
use crate::model::Config;
use crate::selector::Selector;

/// What `--select`, `--extend-select`, `--ignore` and `--preview`/`--no-preview` say, to lay over
/// the configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLine {
    /// Empty when `--select` was not given.
    pub select: Vec<String>,
    pub extend_select: Vec<String>,
    pub ignore: Vec<String>,
    /// `None` leaves what the file says.
    pub preview: Option<bool>,
}

impl Config {
    /// `--select` starts the selection from scratch, as in ruff: it replaces `select`,
    /// `extend-select` and `ignore` of the file. `--extend-select` and `--ignore` are added to
    /// what is left, and `--preview`/`--no-preview` set the opt-in. Anything that names no rule
    /// is an error.
    pub fn with_command_line(
        mut self,
        catalog: &Catalog,
        command_line: &CommandLine,
    ) -> Result<Self, ConfigError> {
        let mut found = Vec::new();
        for (flag, values) in [
            ("--select", &command_line.select),
            ("--extend-select", &command_line.extend_select),
            ("--ignore", &command_line.ignore),
        ] {
            for value in values {
                let verdict = Selector::parse(value).and_then(|selector| selector.check(catalog));
                if let Err(message) = verdict {
                    found.push(issue(flag, message));
                }
            }
        }
        if !found.is_empty() {
            return Err(ConfigError::new(found));
        }
        if !command_line.select.is_empty() {
            self.select = command_line.select.clone();
            self.extend_select.clear();
            self.ignore.clear();
        }
        self.extend_select
            .extend(command_line.extend_select.iter().cloned());
        self.ignore.extend(command_line.ignore.iter().cloned());
        if let Some(preview) = command_line.preview {
            self.preview = preview;
        }
        Ok(self)
    }
}
