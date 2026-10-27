use std::path::Path;

use botica_catalog::{lower_bound, PythonVersion};

/// The version that botica takes for a project whose `requires-python` sets no lower bound: the
/// default of CES-77 for a new repository.
pub const DEFAULT_PYTHON: PythonVersion = PythonVersion::new(3, 14);

/// The Python of a project that the `python` of a rule is compared with: the lowest version that
/// its `requires-python` allows (ADR 0015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPython {
    pub version: PythonVersion,
    /// The `requires-python` of `pyproject.toml`, as written.
    pub requires: Option<String>,
}

impl ProjectPython {
    /// The Python of the project at `root`, from the `requires-python` of its `pyproject.toml`.
    pub fn read(root: &Path) -> Self {
        Self::from_requires(requires_python(root))
    }

    pub fn from_requires(requires: Option<String>) -> Self {
        let version = requires
            .as_deref()
            .and_then(lower_bound)
            .unwrap_or(DEFAULT_PYTHON);
        Self { version, requires }
    }

    /// What the project asks, for the reason of a rule that is not active.
    pub fn describe(&self) -> String {
        match &self.requires {
            Some(requires) if lower_bound(requires).is_some() => {
                format!("the project requires `{requires}`")
            }
            Some(requires) => format!(
                "the `requires-python` of the project, `{requires}`, sets no lower bound, so \
                 botica takes {DEFAULT_PYTHON} (CES-77)"
            ),
            None => format!(
                "the project names no `requires-python`, so botica takes {DEFAULT_PYTHON} (CES-77)"
            ),
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
