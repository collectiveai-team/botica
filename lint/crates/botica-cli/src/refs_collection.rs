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

/// The file of a path under the root of the project, when the path stays inside the project.
pub fn file_of(root: &Path, path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    let inside = !relative.is_absolute()
        && relative
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if inside {
        Ok(root.join(relative))
    } else {
        Err(format!(
            "`{path}` must be a path inside the project, relative to its root"
        ))
    }
}

/// The text of the file at `path` of the project.
pub fn read_text(root: &Path, path: &str) -> Result<String, String> {
    let file = file_of(root, path)?;
    fs::read_to_string(&file).map_err(|error| format!("cannot read `{path}`: {error}"))
}

/// What a reference points at: its path, or its address when it is a page.
pub fn target(reference: &Reference) -> &str {
    reference
        .url
        .as_deref()
        .or(reference.path.as_deref())
        .unwrap_or_default()
}

/// How a reference is named: its address when it is a page; else its path and its lines, with the
/// repository and the commit before them when it is external.
pub fn label(reference: &Reference) -> String {
    let place = match &reference.lines {
        Some(lines) => format!("{}:{lines}", target(reference)),
        None => target(reference).to_owned(),
    };
    match (&reference.repo, &reference.rev) {
        (Some(repo), Some(rev)) => format!("{repo}@{rev} {place}"),
        _ => place,
    }
}
