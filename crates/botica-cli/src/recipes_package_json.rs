//! The packages that `package.json` declares, and the manager of Node packages that the project
//! uses, as `botica recipe check` and `install` read them for a `tools` step of pnpm or npm.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use botica_catalog::tool_package;

const FILE: &str = "package.json";

/// The lists of `package.json` that declare a package.
const LISTS: [&str; 4] = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

/// What the root of the project has as `package.json`.
pub enum PackageJson {
    Missing,
    /// Why it cannot be used: it cannot be read or is not a JSON object.
    Unusable(String),
    /// The packages it declares, by name.
    Declared(BTreeSet<String>),
}

impl PackageJson {
    pub fn read(root: &Path) -> Self {
        let full = root.join(FILE);
        if fs::symlink_metadata(&full).is_err() {
            return Self::Missing;
        }
        let text = match fs::read_to_string(&full) {
            Ok(text) => text,
            Err(error) => return Self::Unusable(format!("cannot read {FILE}: {error}")),
        };
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(serde_json::Value::Object(object)) => Self::Declared(
                LISTS
                    .iter()
                    .filter_map(|list| object.get(*list).and_then(serde_json::Value::as_object))
                    .flat_map(|list| list.keys().cloned())
                    .collect(),
            ),
            _ => Self::Unusable(format!("{FILE} is not a JSON object")),
        }
    }

    /// Why the package of a requirement of `manager` is not declared, when it is not. A package
    /// declared with another range is declared: the project keeps its own (CES-30).
    pub fn problem(&self, manager: &str, requirement: &str) -> Option<String> {
        match self {
            Self::Missing => Some(format!("no {FILE}")),
            Self::Unusable(reason) => Some(reason.clone()),
            Self::Declared(names) => (!names.contains(&tool_package(manager, requirement)))
                .then(|| format!("not in {FILE}")),
        }
    }
}

/// The lockfiles of the managers of Node packages, and the manager of each.
const LOCKFILES: [(&str, &str); 5] = [
    ("pnpm-lock.yaml", "pnpm"),
    ("package-lock.json", "npm"),
    ("yarn.lock", "yarn"),
    ("bun.lock", "bun"),
    ("bun.lockb", "bun"),
];

/// The manager that the project uses: the one of its lockfile, else the one that the field
/// `packageManager` of `package.json` names, else `named`, the manager of the step. Two managers
/// on one `package.json` write two lockfiles, so `install` runs the one that the project has.
pub fn manager(root: &Path, named: &str) -> String {
    if let Some((_, found)) = LOCKFILES.iter().find(|(file, _)| root.join(file).is_file()) {
        return (*found).to_owned();
    }
    let declared = fs::read_to_string(root.join(FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("packageManager")
                .and_then(serde_json::Value::as_str)
                .and_then(|field| field.split('@').next())
                .map(str::to_owned)
        });
    declared.unwrap_or_else(|| named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("botica-package-json-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create the folder");
        dir
    }

    #[test]
    fn a_package_is_found_in_any_list_whatever_its_range() {
        let dir = root("lists");
        fs::write(
            dir.join(FILE),
            r#"{"devDependencies": {"knip": "^6.42.1"}, "dependencies": {"@biomejs/biome": "2.6.2"}}"#,
        )
        .expect("write");

        let declared = PackageJson::read(&dir);

        assert_eq!(declared.problem("pnpm", "knip"), None);
        assert_eq!(declared.problem("npm", "@biomejs/biome@^2"), None);
        assert_eq!(
            declared.problem("pnpm", "typescript").as_deref(),
            Some("not in package.json")
        );
    }

    #[test]
    fn a_file_that_is_not_an_object_cannot_be_used_and_no_file_is_missing() {
        let dir = root("not-object");
        assert_eq!(
            PackageJson::read(&dir).problem("pnpm", "knip").as_deref(),
            Some("no package.json")
        );
        fs::write(dir.join(FILE), "[]").expect("write");

        let reason = PackageJson::read(&dir).problem("pnpm", "knip");

        assert_eq!(reason.as_deref(), Some("package.json is not a JSON object"));
    }

    #[test]
    fn the_lockfile_chooses_the_manager_before_the_field_and_the_step() {
        let dir = root("manager");
        fs::write(dir.join(FILE), r#"{"packageManager": "npm@11.5.1"}"#).expect("write");
        assert_eq!(manager(&dir, "pnpm"), "npm");

        fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").expect("write");
        assert_eq!(manager(&dir, "npm"), "pnpm");

        let bare = root("bare");
        assert_eq!(manager(&bare, "pnpm"), "pnpm");
    }
}
