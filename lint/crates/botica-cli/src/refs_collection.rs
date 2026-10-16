use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Result};
use botica_catalog::{read_references, CatalogError, Reference, References, REFERENCES_DIR};

use crate::project::{self, Project, ProjectArgs};
use crate::refs_standards::{self, Problem, Standard};

/// What `botica refs` reads: the project with the rules of every catalog, the topics of its
/// `references/` and the standards of its `standards/`.
pub struct Collection {
    pub project: Project,
    pub topics: Result<References, CatalogError>,
    pub standards: Vec<Standard>,
    /// The pages of `standards/` whose front matter cannot be read.
    pub standard_problems: Vec<Problem>,
}

pub fn open(args: &ProjectArgs) -> Result<Collection> {
    let project = project::load(args)?;
    let root = project.loaded.root.clone();
    let files = read_references(&root.join(REFERENCES_DIR))?;
    let (standards, standard_problems) = refs_standards::read(&root);
    Ok(Collection {
        project,
        topics: References::from_files(&files),
        standards,
        standard_problems,
    })
}

impl Collection {
    pub fn root(&self) -> &Path {
        &self.project.loaded.root
    }

    /// The topics, when `references/` and `standards/` load; what `list` and `show` need.
    pub fn loaded(&self) -> Result<&References> {
        if let Some(problem) = self.standard_problems.first() {
            bail!("{problem}");
        }
        match &self.topics {
            Ok(topics) => Ok(topics),
            Err(error) => bail!("references/ does not load:\n{error}"),
        }
    }

    /// The rules and the standards that cite a topic: the rules first, in the order of their
    /// codes, then the standards.
    pub fn cited_by(&self, topic: &str) -> Vec<String> {
        let rules = self
            .project
            .catalog
            .rules()
            .iter()
            .filter(|rule| rule.references.iter().any(|cited| cited == topic))
            .map(|rule| rule.id.clone());
        let standards = self
            .standards
            .iter()
            .filter(|standard| standard.references.iter().any(|cited| cited == topic))
            .map(|standard| standard.id.clone());
        rules.chain(standards).collect()
    }
}

/// The file of a reference under the root of the project, when its path stays inside the project.
pub fn file_of(root: &Path, reference: &Reference) -> Result<PathBuf, String> {
    let path = Path::new(&reference.path);
    let inside = !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if inside {
        Ok(root.join(path))
    } else {
        Err(format!(
            "`{}` must be a path inside the project, relative to its root",
            reference.path
        ))
    }
}

/// The text of the file that an internal reference points at.
pub fn read_text(root: &Path, reference: &Reference) -> Result<String, String> {
    let file = file_of(root, reference)?;
    fs::read_to_string(&file).map_err(|error| format!("cannot read `{}`: {error}", reference.path))
}

/// How a reference is named: its path and its lines, with the repository and the commit before
/// them when it is external.
pub fn label(reference: &Reference) -> String {
    let place = match &reference.lines {
        Some(lines) => format!("{}:{lines}", reference.path),
        None => reference.path.clone(),
    };
    match (&reference.repo, &reference.rev) {
        (Some(repo), Some(rev)) => format!("{repo}@{rev} {place}"),
        _ => place,
    }
}
