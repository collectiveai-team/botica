//! The packages that `pyproject.toml` declares, as `botica recipe check` and `install` read them
//! for the `tools` step.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use botica_catalog::normalized_package;

const FILE: &str = "pyproject.toml";

/// What the root of the project has as `pyproject.toml`.
pub enum Pyproject {
    Missing,
    /// Why it cannot be used: it cannot be read or is not TOML.
    Unusable(String),
    /// The packages it declares, by their normalized names.
    Declared(BTreeSet<String>),
}

impl Pyproject {
    pub fn read(root: &Path) -> Self {
        let full = root.join(FILE);
        if fs::symlink_metadata(&full).is_err() {
            return Self::Missing;
        }
        let text = match fs::read_to_string(&full) {
            Ok(text) => text,
            Err(error) => return Self::Unusable(format!("cannot read {FILE}: {error}")),
        };
        match toml::from_str::<toml::Table>(&text) {
            Ok(table) => Self::Declared(declared(&table)),
            Err(_) => Self::Unusable(format!("{FILE} is not valid TOML")),
        }
    }

    /// Why the package is not declared, when it is not.
    pub fn problem(&self, package: &str) -> Option<String> {
        match self {
            Self::Missing => Some(format!("no {FILE}")),
            Self::Unusable(reason) => Some(reason.clone()),
            Self::Declared(names) => {
                (!names.contains(&normalized_package(package))).then(|| format!("not in {FILE}"))
            }
        }
    }
}

/// The names of the requirements in the places where `pyproject.toml` lists them:
/// `project.dependencies`, `project.optional-dependencies`, `dependency-groups` and
/// `tool.uv.dev-dependencies`.
fn declared(table: &toml::Table) -> BTreeSet<String> {
    let mut lists: Vec<&toml::value::Array> = Vec::new();
    let project = table.get("project").and_then(toml::Value::as_table);
    lists.extend(
        project
            .and_then(|project| project.get("dependencies"))
            .and_then(toml::Value::as_array),
    );
    for key in [
        project.and_then(|project| project.get("optional-dependencies")),
        table.get("dependency-groups"),
    ] {
        if let Some(groups) = key.and_then(toml::Value::as_table) {
            lists.extend(groups.values().filter_map(toml::Value::as_array));
        }
    }
    lists.extend(
        table
            .get("tool")
            .and_then(|tool| tool.get("uv"))
            .and_then(|uv| uv.get("dev-dependencies"))
            .and_then(toml::Value::as_array),
    );
    lists
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .filter_map(requirement_name)
        .collect()
}

/// The normalized name at the start of a requirement such as `ruff[x]>=0.5 ; python_version > '3'`.
fn requirement_name(requirement: &str) -> Option<String> {
    let name: String = requirement
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    (!name.is_empty()).then(|| normalized_package(&name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("botica-pyproject-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the folder");
        dir
    }

    #[test]
    fn a_file_that_is_not_toml_cannot_be_used() {
        let dir = root("not-toml");
        fs::write(dir.join(FILE), "= broken").expect("write");

        let reason = Pyproject::read(&dir).problem("ruff");

        assert_eq!(reason.as_deref(), Some("pyproject.toml is not valid TOML"));
    }

    #[test]
    fn a_file_that_cannot_be_read_cannot_be_used() {
        let dir = root("bytes");
        fs::write(dir.join(FILE), [0xff, 0xfe, 0x00]).expect("write");

        let reason = Pyproject::read(&dir).problem("ruff");

        assert!(reason.is_some_and(|text| text.starts_with("cannot read pyproject.toml: ")),);
    }

    #[test]
    fn a_folder_cannot_be_used_and_no_file_is_missing() {
        let dir = root("folder");
        assert_eq!(
            Pyproject::read(&dir).problem("ruff").as_deref(),
            Some("no pyproject.toml")
        );
        fs::create_dir(dir.join(FILE)).expect("create the folder");

        let reason = Pyproject::read(&dir).problem("ruff");

        assert!(reason.is_some_and(|text| text.starts_with("cannot read pyproject.toml: ")));
    }

    #[test]
    fn a_package_is_found_whatever_its_spelling() {
        let dir = root("spelling");
        fs::write(dir.join(FILE), "[dependency-groups]\ndev = [\"my-pkg\"]\n").expect("write");

        let pyproject = Pyproject::read(&dir);

        assert_eq!(pyproject.problem("My_Pkg"), None);
        assert_eq!(
            pyproject.problem("other").as_deref(),
            Some("not in pyproject.toml")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_nothing_cannot_be_used_and_is_not_missing() {
        let dir = root("dangling");
        std::os::unix::fs::symlink("nowhere", dir.join(FILE)).expect("link");

        let reason = Pyproject::read(&dir).problem("ruff");

        assert!(reason.is_some_and(|text| text.starts_with("cannot read pyproject.toml: ")));
    }

    #[test]
    fn the_name_of_a_requirement_ends_before_its_extras_and_specifiers() {
        for (requirement, name) in [
            ("ruff", Some("ruff")),
            ("  Ruff>=0.5", Some("ruff")),
            ("my_pkg[extra]==1", Some("my-pkg")),
            ("a.b ; python_version > '3'", Some("a-b")),
            ("", None),
            (">=1", None),
        ] {
            assert_eq!(
                requirement_name(requirement).as_deref(),
                name,
                "{requirement}"
            );
        }
    }
}
