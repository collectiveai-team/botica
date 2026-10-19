use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// A standard of `standards/<set>/<number>-<slug>.md`, as far as `refs` reads it: the `id` of its
/// front matter and the topics that its `references` field cites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standard {
    pub id: String,
    /// The path of the page from the root of the project.
    pub path: String,
    pub references: Vec<String>,
}

/// Something wrong in a file of the project, as `<path>: <message>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub path: String,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

const REFERENCES_HINT: &str =
    "`references` must be a JSON list of topic ids, like [\"structured-logging\"]";

/// The standards under `standards/` of the project and the pages whose front matter cannot be
/// read. `index.md` of a set is not a standard.
pub fn read(root: &Path) -> (Vec<Standard>, Vec<Problem>) {
    let mut standards = Vec::new();
    let mut problems = Vec::new();
    for page in pages(&root.join("standards")) {
        let path = page
            .strip_prefix(root)
            .unwrap_or(&page)
            .to_string_lossy()
            .replace('\\', "/");
        let parsed = fs::read_to_string(&page)
            .map_err(|error| format!("cannot read the page: {error}"))
            .and_then(|text| parse(&text));
        match parsed {
            Ok((id, references)) => standards.push(Standard {
                id,
                path,
                references,
            }),
            Err(message) => problems.push(Problem { path, message }),
        }
    }
    (standards, problems)
}

fn sorted(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    paths
}

fn pages(standards: &Path) -> Vec<PathBuf> {
    sorted(standards)
        .into_iter()
        .filter(|set| set.is_dir())
        .flat_map(|set| sorted(&set))
        .filter(|page| {
            page.extension().is_some_and(|ext| ext == "md")
                && page.file_name().is_some_and(|name| name != "index.md")
        })
        .collect()
}

fn parse(text: &str) -> Result<(String, Vec<String>), String> {
    let head = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(head, _)| head)
        .ok_or("the page has no front matter")?;
    let field = |key: &str| head.lines().find_map(|line| line.strip_prefix(key));
    let id = field("id: ").ok_or("the front matter has no `id`")?;
    let references = match field("references: ") {
        None => Vec::new(),
        Some(raw) => serde_json::from_str(raw).map_err(|_| REFERENCES_HINT)?,
    };
    Ok((id.to_owned(), references))
}
