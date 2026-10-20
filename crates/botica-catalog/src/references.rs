use std::collections::BTreeMap;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;
use crate::references_check::check_reference;
use crate::validate::is_kebab_case;

/// The folder of the repository that holds the topics, beside `standards/`.
pub const REFERENCES_DIR: &str = "references";
pub const TOPIC_FILE: &str = "topic.toml";
const README_FILE: &str = "README.md";

/// The folder of a topic that holds its code, the files that a `file` reference shows.
pub const CODE_DIR: &str = "code";

/// What a topic points at: a file of the code of the topic (`file`), a file of the repository
/// itself, a file of another repository at a commit, or a page of the web (`url`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// The repository of an external reference; absent in an internal one.
    pub repo: Option<String>,
    /// The commit of an external reference, so that its lines never change.
    pub rev: Option<String>,
    pub license: Option<String>,
    /// A file of the topic, from the folder of the topic: `code/core/logger.py`.
    pub file: Option<String>,
    /// The file that the reference shows; absent in a page reference.
    pub path: Option<String>,
    /// The address of the page that a page reference shows; absent in a file reference.
    pub url: Option<String>,
    /// `"18-30"` or `"18"`: the lines of the file that the reference shows. Absent, the whole file.
    pub lines: Option<String>,
    /// Text that the lines hold, found again when the reference is updated.
    pub anchor: Option<String>,
    /// The passage of a page that matters, as the page words it; absent in a file reference.
    pub quote: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicFile {
    title: String,
    #[serde(default)]
    references: Vec<Reference>,
}

/// Where a topic comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Carried by the binary, from `catalog/references/` of botica.
    BuiltIn,
    /// The `references/` folder of the project.
    Project,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    /// The name of its folder.
    pub id: String,
    pub origin: Origin,
    pub title: String,
    pub references: Vec<Reference>,
    pub readme: String,
    /// The files under `code/`, by their path from the folder of the topic.
    pub files: BTreeMap<String, String>,
}

impl Topic {
    /// The text of the file of the topic that a `file` reference names.
    pub fn file(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }
}

#[derive(Debug, Clone, Default)]
pub struct References {
    pub(crate) topics: Vec<Topic>,
}

impl Reference {
    pub fn is_external(&self) -> bool {
        self.repo.is_some()
    }

    pub fn is_page(&self) -> bool {
        self.url.is_some()
    }

    /// Whether the reference shows a file of the code of its topic.
    pub fn is_topic_file(&self) -> bool {
        self.file.is_some()
    }

    /// What the catalog refuses in the reference: the problems that loading a topic reports for it.
    pub fn problems(&self) -> Vec<String> {
        check_reference(self)
    }

    /// What a reference that points at a whole file or a whole page could name instead: a block of
    /// lines, a `#fragment` of the page or a `quote` of it. It is advice, and the catalog loads
    /// the reference without it. A file of the topic is written for the topic, so the whole file
    /// is what it shows.
    pub fn suggestion(&self) -> Option<String> {
        if self.is_topic_file() {
            return None;
        }
        if let Some(url) = &self.url {
            let fragment = url
                .split_once('#')
                .is_some_and(|(_, rest)| !rest.is_empty());
            return (!fragment && self.quote.is_none()).then(|| {
                "points at the whole page: end the `url` with a `#fragment` of the section, or add a \
                 `quote` of the passage"
                    .to_owned()
            });
        }
        self.lines.is_none().then(|| {
            "points at the whole file: add `lines` for the block that matters, or one reference \
             for each block"
                .to_owned()
        })
    }

    /// The first and the last line of `lines`, counted from 1. `from_files` rejects a reference
    /// whose `lines` is not a range.
    pub fn line_range(&self) -> Option<(usize, usize)> {
        parse_lines(self.lines.as_deref()?)
    }

    /// Whether `text`, the content of the file, still holds what the reference points at: the
    /// lines exist and the anchor is inside them.
    pub fn check(&self, text: &str) -> Result<(), String> {
        let total = text.lines().count();
        if let Some((first, last)) = self.line_range() {
            if last > total {
                return Err(format!(
                    "lines {} are past the end of the file, which has {total} lines",
                    self.lines.as_deref().unwrap_or_default()
                ));
            }
            return self.check_anchor(&self.excerpt(text), &format!("lines {first}-{last}"));
        }
        self.check_anchor(text, "the file")
    }

    fn check_anchor(&self, within: &str, place: &str) -> Result<(), String> {
        match &self.anchor {
            Some(anchor) if !within.contains(anchor.as_str()) => {
                Err(format!("the anchor `{anchor}` is not in {place}"))
            }
            _ => Ok(()),
        }
    }

    /// The lines of `text` that the reference shows, or all of it when it names no lines.
    pub fn excerpt(&self, text: &str) -> String {
        let Some((first, last)) = self.line_range() else {
            return text.to_owned();
        };
        text.split_inclusive('\n')
            .skip(first - 1)
            .take(last - first + 1)
            .collect()
    }
}

pub(crate) fn parse_lines(raw: &str) -> Option<(usize, usize)> {
    let (first, last) = match raw.split_once('-') {
        Some((first, last)) => (first, last),
        None => (raw, raw),
    };
    let (first, last) = (first.parse::<usize>().ok()?, last.parse::<usize>().ok()?);
    (first >= 1 && first <= last).then_some((first, last))
}

#[derive(Default)]
struct TopicEntry<'a> {
    toml: Option<&'a SourceFile>,
    readme: Option<&'a SourceFile>,
    /// The files under `code/`, by their path from the folder of the topic.
    code: BTreeMap<String, &'a SourceFile>,
}

impl References {
    /// Loads the topics from the files of `references/`, with paths from that folder:
    /// `<topic>/topic.toml`, `<topic>/README.md` and the files under `<topic>/code/`.
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        Self::load(files, Origin::Project)
    }

    /// Loads the topics that a binary carries. They point at their own code, external files and
    /// pages only: a file of the project is not there to read.
    pub fn from_builtin_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        Self::load(files, Origin::BuiltIn)
    }

    fn load(files: &[SourceFile], origin: Origin) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let mut entries: BTreeMap<&str, TopicEntry> = BTreeMap::new();
        for file in files {
            match file.path.split('/').collect::<Vec<_>>().as_slice() {
                [id, TOPIC_FILE] => entries.entry(id).or_default().toml = Some(file),
                [id, README_FILE] => entries.entry(id).or_default().readme = Some(file),
                [id, CODE_DIR, rest @ ..] if !rest.is_empty() => {
                    let path = format!("{CODE_DIR}/{}", rest.join("/"));
                    entries.entry(id).or_default().code.insert(path, file);
                }
                _ => issues.push(issue(
                    file.path.as_str(),
                    "unexpected file: expected <topic>/{topic.toml,README.md} or a file under \
                     <topic>/code/",
                )),
            }
        }
        let topics = entries
            .into_iter()
            .filter_map(|(id, entry)| load_topic(id, origin, &entry, &mut issues))
            .collect();
        if issues.is_empty() {
            Ok(Self { topics })
        } else {
            Err(CatalogError { issues })
        }
    }

    pub fn topics(&self) -> &[Topic] {
        &self.topics
    }

    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|topic| topic.id == id)
    }
}

fn load_topic(
    id: &str,
    origin: Origin,
    entry: &TopicEntry<'_>,
    issues: &mut Vec<CatalogIssue>,
) -> Option<Topic> {
    if !is_kebab_case(id) {
        issues.push(issue(
            format!("{id}/{TOPIC_FILE}"),
            format!("folder `{id}` must be a kebab-case topic id"),
        ));
    }
    let Some(toml) = entry.toml else {
        issues.push(issue(format!("{id}/{TOPIC_FILE}"), "missing topic.toml"));
        return None;
    };
    let Some(readme) = entry.readme else {
        issues.push(issue(format!("{id}/{README_FILE}"), "missing README.md"));
        return None;
    };
    let parsed: TopicFile = match toml::from_str(&toml.text) {
        Ok(parsed) => parsed,
        Err(error) => {
            issues.push(issue(toml.path.as_str(), error.message().to_owned()));
            return None;
        }
    };
    let files: BTreeMap<String, String> = entry
        .code
        .iter()
        .map(|(path, file)| (path.clone(), file.text.clone()))
        .collect();
    for (position, reference) in parsed.references.iter().enumerate() {
        let mut problems = check_reference(reference);
        if origin == Origin::BuiltIn && reference.path.is_some() && !reference.is_external() {
            problems.push(
                "a built-in topic points at its own code, external files and pages only".to_owned(),
            );
        }
        if let Some(file) = &reference.file {
            match files.get(file) {
                Some(text) => problems.extend(reference.check(text).err()),
                None if file.starts_with(&format!("{CODE_DIR}/")) => {
                    problems.push(format!("`{file}` is not a file of the topic"));
                }
                None => {}
            }
        }
        for message in problems {
            issues.push(issue(
                toml.path.as_str(),
                format!("references[{position}]: {message}"),
            ));
        }
    }
    Some(Topic {
        id: id.to_owned(),
        origin,
        title: parsed.title,
        references: parsed.references,
        readme: readme.text.clone(),
        files,
    })
}
