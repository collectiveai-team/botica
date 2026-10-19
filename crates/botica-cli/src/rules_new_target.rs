//! The catalog that `rules new` writes a rule into: the built-in one, or one catalog of a project.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use botica_catalog::{read_files, read_project_files, Catalog, Family, SourceFile, CATALOG_FILE};

pub struct Target {
    /// The name of the catalog of a project; `None` for the built-in catalog.
    pub project: Option<String>,
    pub catalog: Catalog,
    /// The files that the catalog loads from, with the paths its loader takes.
    existing: Vec<SourceFile>,
}

impl Target {
    pub fn load(dir: &Path) -> Result<Self> {
        if !dir.join(CATALOG_FILE).is_file() {
            let existing = read_files(dir)
                .with_context(|| format!("cannot read the catalog `{}`", dir.display()))?;
            let catalog = Catalog::from_files(&existing)?;
            return Ok(Self {
                project: None,
                existing,
                catalog,
            });
        }
        let dir = fs::canonicalize(dir)
            .with_context(|| format!("cannot read the catalog `{}`", dir.display()))?;
        let name = dir.file_name().and_then(|name| name.to_str());
        let (name, rules_dir) = name
            .zip(dir.parent())
            .with_context(|| format!("`{}` has no name of a catalog", dir.display()))?;
        let existing = read_project_files(rules_dir)
            .with_context(|| format!("cannot read the catalogs in `{}`", rules_dir.display()))?;
        let catalog = Catalog::embedded()?
            .with_projects(&existing)
            .with_context(|| format!("the catalogs in `{}` do not load", rules_dir.display()))?;
        Ok(Self {
            project: Some(name.to_owned()),
            existing,
            catalog,
        })
    }

    /// The id of the rule in the folder `slug`.
    pub fn id(&self, slug: &str) -> String {
        match &self.project {
            Some(name) => format!("{name}/{slug}"),
            None => slug.to_owned(),
        }
    }

    pub fn families(&self) -> impl Iterator<Item = &Family> {
        self.catalog
            .families()
            .iter()
            .filter(|family| family.catalog == self.project)
    }

    /// Whether the catalog loads with the files of a rule, with paths from the catalog.
    pub fn loads_with(&self, files: &[SourceFile]) -> Result<()> {
        match &self.project {
            None => {
                Catalog::from_files(&[self.existing.clone(), files.to_vec()].concat())?;
            }
            Some(name) => {
                let prefixed: Vec<SourceFile> = files
                    .iter()
                    .map(|file| SourceFile {
                        path: format!("{name}/{}", file.path),
                        text: file.text.clone(),
                    })
                    .collect();
                Catalog::embedded()?.with_projects(&[self.existing.clone(), prefixed].concat())?;
            }
        }
        Ok(())
    }
}
