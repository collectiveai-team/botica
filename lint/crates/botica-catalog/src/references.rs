use std::collections::BTreeMap;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;
use crate::validate::is_kebab_case;

/// The folder of the repository that holds the topics, beside `standards/`.
pub const REFERENCES_DIR: &str = "references";
pub const TOPIC_FILE: &str = "topic.toml";
const README_FILE: &str = "README.md";

/// A piece of code that a topic points at: a file of the repository itself, or a file of another
/// repository at a commit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// The repository of an external reference; absent in an internal one.
    pub repo: Option<String>,
    /// The commit of an external reference, so that its lines never change.
    pub rev: Option<String>,
    pub license: Option<String>,
    pub path: String,
    /// `"18-30"` or `"18"`: the lines of the file that the reference shows. Absent, the whole file.
    pub lines: Option<String>,
    /// Text that the lines hold, found again when the reference is updated.
    pub anchor: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopicFile {
    title: String,
    #[serde(default)]
    docs: Vec<String>,
    #[serde(default)]
    references: Vec<Reference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    /// The name of its folder.
    pub id: String,
    pub title: String,
    /// The addresses of the documentation of the libraries the topic uses.
    pub docs: Vec<String>,
    pub references: Vec<Reference>,
    pub readme: String,
}

#[derive(Debug, Clone, Default)]
pub struct References {
    topics: Vec<Topic>,
}

impl Reference {
    pub fn is_external(&self) -> bool {
        self.repo.is_some()
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

fn parse_lines(raw: &str) -> Option<(usize, usize)> {
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
}

impl References {
    /// Loads the topics from the files of `references/`, with paths from that folder:
    /// `<topic>/topic.toml` and `<topic>/README.md`.
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let mut entries: BTreeMap<&str, TopicEntry> = BTreeMap::new();
        for file in files {
            match file.path.split('/').collect::<Vec<_>>().as_slice() {
                [id, TOPIC_FILE] => entries.entry(id).or_default().toml = Some(file),
                [id, README_FILE] => entries.entry(id).or_default().readme = Some(file),
                _ => issues.push(issue(
                    file.path.as_str(),
                    "unexpected file: expected <topic>/{topic.toml,README.md}",
                )),
            }
        }
        let topics = entries
            .into_iter()
            .filter_map(|(id, entry)| load_topic(id, &entry, &mut issues))
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

fn load_topic(id: &str, entry: &TopicEntry<'_>, issues: &mut Vec<CatalogIssue>) -> Option<Topic> {
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
    for (position, reference) in parsed.references.iter().enumerate() {
        for message in check_reference(reference) {
            issues.push(issue(
                toml.path.as_str(),
                format!("references[{position}]: {message}"),
            ));
        }
    }
    Some(Topic {
        id: id.to_owned(),
        title: parsed.title,
        docs: parsed.docs,
        references: parsed.references,
        readme: readme.text.clone(),
    })
}

fn check_reference(reference: &Reference) -> Vec<String> {
    let mut problems = Vec::new();
    if reference.path.trim().is_empty() {
        problems.push("`path` is empty".to_owned());
    }
    if let Some(lines) = &reference.lines {
        if parse_lines(lines).is_none() {
            problems.push(format!(
                "`lines` must be `<first>-<last>` or `<line>`, counted from 1 (got `{lines}`)"
            ));
        }
    }
    match (&reference.repo, &reference.rev) {
        (Some(_), None) => {
            problems.push("`repo` needs a `rev`: the commit that fixes the lines".to_owned())
        }
        (None, Some(_)) => problems.push("`rev` needs a `repo`".to_owned()),
        _ => {}
    }
    match (&reference.repo, &reference.license) {
        (Some(_), None) => problems.push("an external reference needs a `license`".to_owned()),
        (None, Some(_)) => problems.push("`license` needs a `repo`".to_owned()),
        _ => {}
    }
    problems
}
