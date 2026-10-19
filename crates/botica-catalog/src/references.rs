use std::collections::BTreeMap;

use serde::Deserialize;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::load::SourceFile;
use crate::validate::is_kebab_case;

/// The folder of the repository that holds the topics, beside `standards/`.
pub const REFERENCES_DIR: &str = "references";
pub const TOPIC_FILE: &str = "topic.toml";
const README_FILE: &str = "README.md";

/// What a topic points at: a file of the repository itself, a file of another repository at a
/// commit, or a page of the web (`url`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// The repository of an external reference; absent in an internal one.
    pub repo: Option<String>,
    /// The commit of an external reference, so that its lines never change.
    pub rev: Option<String>,
    pub license: Option<String>,
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

    /// What the catalog refuses in the reference: the problems that loading a topic reports for it.
    pub fn problems(&self) -> Vec<String> {
        check_reference(self)
    }

    /// What a reference that points at a whole file or a whole page could name instead: a block of
    /// lines, a `#fragment` of the page or a `quote` of it. It is advice, and the catalog loads
    /// the reference without it.
    pub fn suggestion(&self) -> Option<String> {
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
        Self::load(files, Origin::Project)
    }

    /// Loads the topics that a binary carries. They point at external files and pages only: a
    /// file of the project is not there to read.
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
                _ => issues.push(issue(
                    file.path.as_str(),
                    "unexpected file: expected <topic>/{topic.toml,README.md}",
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
    for (position, reference) in parsed.references.iter().enumerate() {
        let mut problems = check_reference(reference);
        if origin == Origin::BuiltIn && reference.path.is_some() && !reference.is_external() {
            problems.push("a built-in topic points at external files and pages only".to_owned());
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
    })
}

fn check_reference(reference: &Reference) -> Vec<String> {
    let mut problems = Vec::new();
    match (&reference.path, &reference.url) {
        (None, None) => problems.push("needs a `path` or a `url`".to_owned()),
        (Some(_), Some(_)) => problems.push("`path` and `url` exclude each other".to_owned()),
        _ => {}
    }
    if reference
        .path
        .as_deref()
        .is_some_and(|path| path.trim().is_empty())
    {
        problems.push("`path` is empty".to_owned());
    }
    if let Some(url) = &reference.url {
        problems.extend(check_page(reference, url));
        return problems;
    }
    if reference.quote.is_some() {
        problems
            .push("`quote` does not apply to a file reference: the lines are its text".to_owned());
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
    if reference
        .repo
        .as_deref()
        .is_some_and(|repo| !is_repo_address(repo))
    {
        problems.push(
            "`repo` must be `<host>/<path>`, with no scheme and no user (`github.com/<owner>/<repo>`)"
                .to_owned(),
        );
    }
    if reference
        .rev
        .as_deref()
        .is_some_and(|rev| !is_commit_sha(rev))
    {
        problems.push(
            "`rev` must be a full commit SHA: 40 lowercase hex digits (64 in a SHA-256 \
             repository), not a branch, a tag or an abbreviation"
                .to_owned(),
        );
    }
    problems
}

fn check_page(reference: &Reference, url: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_http_address(url) {
        problems.push("`url` must be an `https://` or `http://` address".to_owned());
    }
    let present = [
        ("lines", reference.lines.is_some()),
        ("anchor", reference.anchor.is_some()),
        ("repo", reference.repo.is_some()),
        ("rev", reference.rev.is_some()),
        ("license", reference.license.is_some()),
    ];
    for (field, _) in present.iter().filter(|(_, present)| *present) {
        problems.push(format!("`{field}` does not apply to a `url` reference"));
    }
    if reference
        .quote
        .as_deref()
        .is_some_and(|quote| quote.trim().is_empty())
    {
        problems.push("`quote` is empty".to_owned());
    }
    problems
}

fn is_http_address(url: &str) -> bool {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

fn is_commit_sha(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `<host>/<path>`: a host and at least one segment, with no scheme, no user and no port.
fn is_repo_address(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let host = parts.next().unwrap_or_default();
    let segments: Vec<&str> = parts.collect();
    let host_ok = !host.is_empty()
        && !host.starts_with(['-', '.'])
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'));
    host_ok && !segments.is_empty() && segments.iter().all(|part| is_path_segment(part))
}

fn is_path_segment(part: &str) -> bool {
    !part.is_empty()
        && !matches!(part, "." | "..")
        && !part.starts_with('-')
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
