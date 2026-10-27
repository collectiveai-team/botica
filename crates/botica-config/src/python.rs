use std::path::Path;

use botica_catalog::{lower_bound, PythonRange, PythonVersion};

/// The Python of a project that the `python` of a rule is compared with: the lowest version that
/// its `requires-python` allows (ADR 0015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPython {
    /// `None` when `requires-python` is missing, sets no lower bound or cannot be read.
    pub version: Option<PythonVersion>,
    /// The `requires-python` of `pyproject.toml`, as written.
    pub requires: Option<String>,
}

impl ProjectPython {
    /// The Python of the project at `root`, from the `requires-python` of its `pyproject.toml`.
    pub fn read(root: &Path) -> Self {
        Self::from_requires(requires_python(root))
    }

    pub fn from_requires(requires: Option<String>) -> Self {
        let version = requires.as_deref().and_then(lower_bound);
        Self { version, requires }
    }

    /// Whether `range` holds the Python of the project. A project with no lower bound counts as
    /// older than every version a range names (ADR 0015).
    pub fn is_in(&self, range: &PythonRange) -> bool {
        match self.version {
            Some(version) => range.contains(version),
            None => range.holds_an_older_version(),
        }
    }

    /// What the project asks, for the reason of a rule that is not active.
    pub fn describe(&self) -> String {
        match (&self.requires, self.version) {
            (Some(requires), Some(_)) => format!("the project requires `{requires}`"),
            (Some(requires), None) => {
                format!("the `requires-python` of the project, `{requires}`, sets no lower bound")
            }
            (None, _) => "the project names no lower bound in `requires-python`".to_owned(),
        }
    }
}

/// The `requires-python` of the `[project]` of `pyproject.toml` at `root`, trimmed.
pub fn requires_python(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("pyproject.toml")).ok()?;
    let document: toml::Table = text.parse().ok()?;
    let project = document.get("project")?.as_table()?;
    Some(project.get("requires-python")?.as_str()?.trim().to_owned())
}
