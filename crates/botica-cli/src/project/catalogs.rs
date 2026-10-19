//! Where the catalogs of a project are: the folders that `rule-dirs` names, each folder in them a
//! catalog, and the catalogs that `[catalogs]` declares by path.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use botica_catalog::{project_catalog_dirs, read_catalog_files, SourceFile};
use botica_config::Config;

/// The files of the catalogs of a project, and the folder of each, by catalog name.
pub struct ProjectCatalogs {
    pub files: Vec<SourceFile>,
    pub dirs: BTreeMap<String, PathBuf>,
    /// The folders of the catalogs inside the project that the walk does not enter, as globs of
    /// `extend-exclude`: their fixtures are examples of what a rule flags.
    pub excluded: Vec<String>,
}

/// The folder as a glob from the root of the project, when it is inside it.
fn exclusion(root: &Path, dir: &Path) -> Option<String> {
    let inside = dir.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for part in inside.components() {
        let Component::Normal(name) = part else {
            return None;
        };
        parts.push(name.to_string_lossy());
    }
    (!parts.is_empty()).then(|| format!("{}/", parts.join("/")))
}

pub fn read(root: &Path, config: &Config) -> Result<ProjectCatalogs> {
    let mut dirs: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut add = |name: String, dir: PathBuf| -> Result<()> {
        match dirs.insert(name.clone(), dir) {
            Some(first) => bail!(
                "the catalog `{name}` is in the project more than once: `{}` and another folder",
                first.display()
            ),
            None => Ok(()),
        }
    };
    let mut excluded = Vec::new();
    for rule_dir in &config.rule_dirs {
        let folder = root.join(rule_dir);
        let listed = project_catalog_dirs(&folder)
            .with_context(|| format!("cannot read the catalogs of the project in `{rule_dir}`"))?;
        excluded.extend(exclusion(root, &folder));
        for (name, dir) in listed {
            add(name, dir)?;
        }
    }
    for (name, source) in &config.catalogs {
        let dir = root.join(&source.path);
        if !dir.is_dir() {
            bail!(
                "the catalog `{name}` is declared at `{}`, which is not a folder",
                source.path
            );
        }
        excluded.extend(exclusion(root, &dir));
        add(name.clone(), dir)?;
    }
    let mut files = Vec::new();
    for (name, dir) in &dirs {
        files.extend(
            read_catalog_files(name, dir).with_context(|| {
                format!("cannot read the catalog `{name}` in `{}`", dir.display())
            })?,
        );
    }
    Ok(ProjectCatalogs {
        files,
        dirs,
        excluded,
    })
}
