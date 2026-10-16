use std::collections::BTreeMap;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;

pub const TOPIC_FILE: &str = "topic.toml";

/// Where the code of a reference lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The working copy of the repository that holds `references/`.
    Internal,
}

/// A range of lines of a file, counted from 1, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lines {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub place: Place,
    pub path: String,
    pub lines: Option<Lines>,
    pub anchor: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub docs: Vec<String>,
    pub references: Vec<Reference>,
    pub readme: String,
}

#[derive(Debug, Clone, Default)]
pub struct Topics {
    topics: Vec<Topic>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicFile {
    title: String,
    #[serde(default)]
    docs: Vec<String>,
    #[serde(default)]
    references: Vec<ReferenceFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceFile {
    path: String,
    lines: Option<String>,
    anchor: Option<String>,
    note: String,
}

#[derive(Default)]
struct TopicEntry<'a> {
    toml: Option<&'a SourceFile>,
    readme: Option<&'a SourceFile>,
}

impl Topics {
    /// Loads the topics from the files of `references/`, with paths from that folder:
    /// `<topic>/topic.toml` and `<topic>/README.md`.
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let mut entries: BTreeMap<&str, TopicEntry> = BTreeMap::new();
        for file in files {
            let parts: Vec<&str> = file.path.split('/').collect();
            match parts.as_slice() {
                [id, TOPIC_FILE] => entries.entry(id).or_default().toml = Some(file),
                [id, "README.md"] => entries.entry(id).or_default().readme = Some(file),
                _ => {}
            }
        }
        let mut topics = Vec::new();
        for (id, entry) in entries {
            if let Some(topic) = load_topic(id, &entry, &mut issues) {
                topics.push(topic);
            }
        }
        if issues.is_empty() {
            Ok(Self { topics })
        } else {
            Err(CatalogError { issues })
        }
    }

    pub fn topics(&self) -> &[Topic] {
        &self.topics
    }

    pub fn get(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|topic| topic.id == id)
    }
}

fn load_topic(id: &str, entry: &TopicEntry<'_>, issues: &mut Vec<CatalogIssue>) -> Option<Topic> {
    let toml_file = entry.toml?;
    let readme = entry.readme?;
    let parsed: TopicFile = match toml::from_str(&toml_file.text) {
        Ok(parsed) => parsed,
        Err(error) => {
            issues.push(issue(toml_file.path.as_str(), error.message().to_owned()));
            return None;
        }
    };
    let references = parsed
        .references
        .into_iter()
        .filter_map(|reference| load_reference(reference).ok())
        .collect();
    Some(Topic {
        id: id.to_owned(),
        title: parsed.title,
        docs: parsed.docs,
        references,
        readme: readme.text.clone(),
    })
}

fn load_reference(file: ReferenceFile) -> Result<Reference, String> {
    let lines = file.lines.as_deref().map(parse_lines).transpose()?;
    Ok(Reference {
        place: Place::Internal,
        path: file.path,
        lines,
        anchor: file.anchor,
        note: file.note,
    })
}

fn parse_lines(text: &str) -> Result<Lines, String> {
    let (start, end) = text
        .split_once('-')
        .ok_or_else(|| format!("lines `{text}` must look like `10-40`"))?;
    let number = |part: &str| {
        part.parse::<usize>()
            .map_err(|_| format!("lines `{text}` must look like `10-40`"))
    };
    Ok(Lines {
        start: number(start)?,
        end: number(end)?,
    })
}
