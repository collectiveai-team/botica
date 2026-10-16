//! The catalogs of a project, `.botica/rules/<catalog>/`, loaded on top of the built-in one.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::{load_parts, Catalog, Parts, SourceFile, CATALOG_FILE};
use crate::origin::{Origin, HEADER};
use crate::validate::is_kebab_case;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    code: String,
}

/// The files of one catalog folder, with paths from that folder.
#[derive(Default)]
struct Folder {
    toml: Option<SourceFile>,
    files: Vec<SourceFile>,
}

impl Catalog {
    /// This catalog with the catalogs of a project after it. `files` are those of `.botica/rules`,
    /// with paths from that folder: `<catalog>/catalog.toml`, `<catalog>/<family>/family.toml` and
    /// `<catalog>/<family>/<slug>/…`.
    pub fn with_projects(&self, files: &[SourceFile]) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let folders = split(files, &mut issues);
        let codes: Vec<(&str, String)> = folders
            .iter()
            .filter_map(|(name, folder)| Some((*name, read_code(name, folder, &mut issues)?)))
            .collect();
        check_codes(&codes, &mut issues);

        let mut loaded = Vec::new();
        for (name, code) in &codes {
            let origin = Origin::Project { name, code };
            let mut local = Vec::new();
            let parts = load_parts(&folders[name].files, origin, &mut local);
            issues.extend(local.into_iter().map(|found| CatalogIssue {
                path: format!("{name}/{}", found.path),
                message: found.message,
            }));
            loaded.push((*name, parts));
        }
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }

        let mut merged = self.clone();
        for (name, parts) in loaded {
            check_aliases(&merged, name, &parts, &mut issues);
            merged.append(parts);
        }
        if issues.is_empty() {
            Ok(merged)
        } else {
            Err(CatalogError { issues })
        }
    }
}

fn split<'a>(files: &'a [SourceFile], issues: &mut Vec<CatalogIssue>) -> BTreeMap<&'a str, Folder> {
    let mut folders: BTreeMap<&str, Folder> = BTreeMap::new();
    for file in files {
        let Some((name, rest)) = file.path.split_once('/') else {
            issues.push(issue(
                file.path.as_str(),
                "unexpected file: the catalogs of a project are folders, `<catalog>/catalog.toml`",
            ));
            continue;
        };
        let relative = SourceFile {
            path: rest.to_owned(),
            text: file.text.clone(),
        };
        let folder = folders.entry(name).or_default();
        if rest == CATALOG_FILE {
            folder.toml = Some(relative);
        } else {
            folder.files.push(relative);
        }
    }
    folders
}

/// The code a catalog declares in its `catalog.toml`, when the folder and the file are in order.
fn read_code(name: &str, folder: &Folder, issues: &mut Vec<CatalogIssue>) -> Option<String> {
    if !is_kebab_case(name) {
        issues.push(issue(
            name,
            format!("folder `{name}` must be a kebab-case catalog name"),
        ));
        return None;
    }
    let path = format!("{name}/{CATALOG_FILE}");
    let Some(file) = &folder.toml else {
        issues.push(issue(
            path,
            "a catalog needs a catalog.toml with its `code`",
        ));
        return None;
    };
    let parsed: CatalogFile = match toml::from_str(&file.text) {
        Ok(parsed) => parsed,
        Err(error) => {
            issues.push(issue(path, error.message().to_owned()));
            return None;
        }
    };
    if let Some(message) = check_code(&parsed.code) {
        issues.push(issue(path, message));
        return None;
    }
    Some(parsed.code)
}

fn check_code(code: &str) -> Option<String> {
    let letters = !code.is_empty() && code.bytes().all(|b| b.is_ascii_uppercase());
    if !letters {
        return Some(format!(
            "code `{code}` must be one or more letters A-Z (a digit or a separator breaks `noqa` parsing)"
        ));
    }
    (code.starts_with(HEADER) || HEADER.starts_with(code)).then(|| {
        format!(
            "code `{code}` is reserved: `{HEADER}` is the code of the built-in catalog, and no other code starts with it or is a prefix of it"
        )
    })
}

/// The codes of two catalogs differ, and neither is a text prefix of the other: selecting
/// `ACMESTRUC` must name one family.
fn check_codes(codes: &[(&str, String)], issues: &mut Vec<CatalogIssue>) {
    for (name, code) in codes {
        for (other_name, other) in codes.iter().filter(|(other_name, _)| other_name != name) {
            let path = format!("{name}/{CATALOG_FILE}");
            if code == other {
                if name > other_name {
                    issues.push(issue(
                        path,
                        format!("code `{code}` is already the code of the catalog `{other_name}`"),
                    ));
                }
            } else if other.starts_with(code.as_str()) {
                issues.push(issue(
                    path,
                    format!(
                        "code `{code}` is a text prefix of `{other}`, the code of the catalog `{other_name}`: selecting `{code}…` would be ambiguous"
                    ),
                ));
            }
        }
    }
}

/// An alias of a rule of the project is not the id, the code or an alias of a rule that the
/// catalog holds already.
fn check_aliases(merged: &Catalog, name: &str, parts: &Parts, issues: &mut Vec<CatalogIssue>) {
    for (path, rule) in &parts.rules {
        for alias in rule.aliases.iter().filter(|alias| merged.knows(alias)) {
            issues.push(issue(
                format!("{name}/{path}"),
                format!(
                    "alias `{alias}` collides with the id, the code or an alias of another rule"
                ),
            ));
        }
    }
}
