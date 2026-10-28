//! `botica-version-aligned`: the hooks, the workflows and the parameter `botica-version` of a
//! project name one version of botica. `prek update` moves the `rev` of the mirror and leaves the
//! `uvx botica@<version>` lines as they are.

use std::cmp::Ordering;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;

pub const BOTICA_VERSION_ALIGNED: &str = "botica-version-aligned";

/// The mirror of botica for prek and pre-commit, whose `rev` is the version that the others
/// follow.
const MIRROR: &str = "github.com/collectiveai-team/botica-pre-commit";
const PREK: &str = "prek.toml";
const PRE_COMMIT: &str = ".pre-commit-config.yaml";
const WORKFLOWS: &str = ".github/workflows/";
const PIN: &str = "botica@";
const PARAM: &str = "botica-version";

/// A file that the rule reads, from the root of the project, and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The parameter `botica-version` of the recipes in the configuration of botica: the file of the
/// configuration from the root of the project, its text, and the value.
pub struct Param<'a> {
    pub path: &'a str,
    pub text: &'a str,
    pub value: &'a str,
}

/// Whether the rule reads the text of `path`: `prek.toml`, `.pre-commit-config.yaml` and the
/// workflows of GitHub Actions, at the root of the project.
pub fn reads(path: &str) -> bool {
    path == PREK
        || path == PRE_COMMIT
        || path.strip_prefix(WORKFLOWS).is_some_and(|name| {
            !name.contains('/') && (name.ends_with(".yml") || name.ends_with(".yaml"))
        })
}

/// A version of botica that a file names: the version, and the range of its text in bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Pin<'a> {
    pub version: &'a str,
    pub start: usize,
    pub end: usize,
}

/// The `botica@<version>` of `text` outside the lines that are comments. A version starts with a
/// digit, so `botica@latest` and the parameter of a template are none, and the `@` of an action of
/// GitHub (`owner/botica@v1`) or of a longer name follows a `/` or a letter.
pub fn pins(text: &str) -> Vec<Pin<'_>> {
    let mut found = Vec::new();
    for (start, line) in lines(text) {
        if line.trim_start().starts_with('#') {
            continue;
        }
        found.extend(
            line.match_indices(PIN)
                .filter_map(|(at, _)| pin_at(line, at, start)),
        );
    }
    found
}

fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_inclusive('\n').scan(0, |offset, line| {
        let start = *offset;
        *offset += line.len();
        Some((start, line))
    })
}

fn pin_at(line: &str, at: usize, start: usize) -> Option<Pin<'_>> {
    let before = line[..at].chars().next_back();
    if before.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '@'))
    {
        return None;
    }
    let rest = &line[at + PIN.len()..];
    let length = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '!' | '-' | '_')))
        .unwrap_or(rest.len());
    let version = rest[..length].trim_end_matches(['.', '-']);
    is_version(version).then_some(Pin {
        version,
        start: start + at,
        end: start + at + PIN.len() + version.len(),
    })
}

fn is_version(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_digit())
}

fn is_mirror(url: &str) -> bool {
    url.trim_end_matches('/')
        .trim_end_matches(".git")
        .to_ascii_lowercase()
        .ends_with(MIRROR)
}

/// The version that the `rev` of the mirror names in the text of `prek.toml`, without its `v`, or
/// `None` when no repo of the file is the mirror or its `rev` is not a version.
pub fn mirror_version(prek: &str) -> Option<String> {
    let table: toml::Table = toml::from_str(prek).ok()?;
    let repos = table.get("repos")?.as_array()?;
    let rev = repos.iter().find_map(|repo| {
        is_mirror(repo.get("repo")?.as_str()?)
            .then(|| repo.get("rev")?.as_str())
            .flatten()
    })?;
    let version = rev.strip_prefix('v').unwrap_or(rev);
    is_version(version).then(|| version.to_owned())
}

/// The `rev` of the mirror in the text of `.pre-commit-config.yaml`: the version without its `v`,
/// and the range of the `rev` as written. The `rev:` is the first one after the `repo:` of the
/// mirror and before the next item of the list of repos.
pub fn pre_commit_rev(text: &str) -> Option<Pin<'_>> {
    // The column of the `-` of the item of the mirror, while the lines are inside it.
    let mut mirror: Option<usize> = None;
    for (start, line) in lines(text) {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        let item = trimmed.starts_with("- ");
        if item && mirror.is_some_and(|column| indent <= column) {
            mirror = None;
        }
        let Some((key, at, value)) = yaml_entry(line) else {
            continue;
        };
        if key == "repo" {
            let column = if item {
                indent
            } else {
                indent.saturating_sub(2)
            };
            mirror = is_mirror(value).then_some(column);
        } else if mirror.is_some() && key == "rev" {
            let version = value.strip_prefix('v').unwrap_or(value);
            return is_version(version).then_some(Pin {
                version,
                start: start + at,
                end: start + at + value.len(),
            });
        }
    }
    None
}

/// The key of a line `key: value` of YAML, with or without the `- ` of an item, the byte of the
/// line where the value starts, and the value without quotes.
fn yaml_entry(line: &str) -> Option<(&str, usize, &str)> {
    let body = line.trim_start();
    let body = body.strip_prefix("- ").unwrap_or(body).trim_start();
    let (key, rest) = body.split_once(':')?;
    let raw = rest.split(" #").next().unwrap_or(rest).trim();
    let value = raw.trim_matches(['"', '\'']);
    if value.is_empty() || key.contains(' ') {
        return None;
    }
    let at = line.len() - rest.len() + rest.find(value)?;
    Some((key.trim(), at, value))
}

/// Where the value of `botica-version` is written in the text of the configuration, with its
/// quotes: a key `botica-version`, bare or quoted, at the start of a line or of an inline table.
fn param_value(text: &str, value: &str) -> Option<(usize, usize)> {
    for (start, line) in lines(text) {
        for (at, _) in line.match_indices(PARAM) {
            let before = line[..at].trim_end_matches(['"', '\'']).trim_end();
            if !(before.is_empty() || before.ends_with(['{', ','])) {
                continue;
            }
            let rest = line[at + PARAM.len()..].trim_start_matches(['"', '\'']);
            let Some(rest) = rest.trim_start().strip_prefix('=') else {
                continue;
            };
            let written = rest.trim_start();
            let length = ["\"", "'"]
                .iter()
                .find_map(|quote| {
                    let quoted = format!("{quote}{value}{quote}");
                    written.starts_with(&quoted).then_some(quoted.len())
                })
                .or_else(|| written.starts_with(value).then_some(value.len()))?;
            let begin = start + line.len() - written.len();
            return Some((begin, begin + length));
        }
    }
    None
}

/// The version that the others follow, and what the message says of it.
struct Reference {
    version: String,
    said: String,
}

impl Reference {
    fn mirror(file: &str, version: &str) -> Self {
        Self {
            version: version.to_owned(),
            said: format!(
                "the `rev` of the mirror `collectiveai-team/botica-pre-commit` in {file} is \
                 `v{version}`"
            ),
        }
    }
}

/// The findings of the rule over `sources` and the parameter `botica-version` of the
/// configuration, when it has one. The reference is the `rev` of the mirror in `prek.toml`, then
/// the one in `.pre-commit-config.yaml`, then the newest version that the files name. A
/// `botica@<version>` with another version is reported, the `rev` of `.pre-commit-config.yaml`
/// when `prek.toml` is the reference and names another version, and the parameter.
pub fn check(
    sources: &[Source<'_>],
    param: Option<&Param<'_>>,
    severity: Severity,
) -> Vec<Diagnostic> {
    let text_of = |path: &str| {
        sources
            .iter()
            .find(|source| source.path == path)
            .map(|source| source.text)
    };
    let prek = text_of(PREK).and_then(mirror_version);
    let pre_commit = sources
        .iter()
        .find(|source| source.path == PRE_COMMIT)
        .and_then(|source| Some((source, pre_commit_rev(source.text)?)));
    let found: Vec<(&Source<'_>, Pin<'_>)> = sources
        .iter()
        .flat_map(|source| pins(source.text).into_iter().map(move |pin| (source, pin)))
        .collect();
    let reference = match (&prek, &pre_commit) {
        (Some(version), _) => Reference::mirror(PREK, version),
        (None, Some((_, rev))) => Reference::mirror(PRE_COMMIT, rev.version),
        (None, None) => match found
            .iter()
            .map(|(_, pin)| pin.version)
            .max_by(|a, b| newer(a, b))
        {
            Some(version) => Reference {
                version: version.to_owned(),
                said: format!("the newest version that the project names is {version}"),
            },
            None => return Vec::new(),
        },
    };
    let wanted = reference.version.as_str();
    let said = &reference.said;
    let mut diagnostics: Vec<Diagnostic> = found
        .iter()
        .filter(|(_, pin)| pin.version != wanted)
        .map(|(source, pin)| {
            let message = format!(
                "This line runs botica {}, and {said}: write `botica@{wanted}` here, so that the \
                 hooks and the workflows run one version of botica",
                pin.version
            );
            on_its_line(
                source.path,
                source.text,
                pin.start,
                pin.end,
                severity,
                message,
            )
        })
        .collect();
    if let (Some(_), Some((source, rev))) = (&prek, &pre_commit) {
        if rev.version != wanted {
            let message = format!(
                "This `rev` installs botica {}, and {said}: write `v{wanted}` here, so that prek \
                 and pre-commit install one version of botica",
                rev.version
            );
            diagnostics.push(on_its_line(
                source.path,
                source.text,
                rev.start,
                rev.end,
                severity,
                message,
            ));
        }
    }
    if let Some(param) = param.filter(|param| param.value != wanted) {
        let message = format!(
            "The parameter `botica-version = \"{}\"` of the recipes makes the next `botica recipe \
             install` write `botica@{}` again, and {said}: set it to `{wanted}`, or remove it so \
             that `install` writes the version of the botica that runs it",
            param.value, param.value
        );
        diagnostics.push(match param_value(param.text, param.value) {
            Some((start, end)) => {
                on_its_line(param.path, param.text, start, end, severity, message)
            }
            None => Diagnostic::file_level(param.path, BOTICA_VERSION_ALIGNED, severity, message),
        });
    }
    diagnostics
}

/// Orders two versions by their numbers; a version with something after its numbers (`0.18.0rc1`)
/// comes before the same numbers alone.
fn newer(a: &str, b: &str) -> Ordering {
    let (a_numbers, a_rest) = split(a);
    let (b_numbers, b_rest) = split(b);
    let length = a_numbers.len().max(b_numbers.len());
    let at = |numbers: &[u64], i: usize| numbers.get(i).copied().unwrap_or(0);
    (0..length)
        .map(|i| at(&a_numbers, i).cmp(&at(&b_numbers, i)))
        .find(|order| order.is_ne())
        .unwrap_or_else(|| match (a_rest.is_empty(), b_rest.is_empty()) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => a_rest.cmp(b_rest),
        })
}

fn split(version: &str) -> (Vec<u64>, &str) {
    let end = version
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(version.len());
    let numbers = version[..end]
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    (numbers, &version[end..])
}

fn on_its_line(
    path: &str,
    text: &str,
    start: usize,
    end: usize,
    severity: Severity,
    message: String,
) -> Diagnostic {
    let document = Document::new(path, text);
    let (line, column) = document.position(start);
    let (end_line, end_column) = document.position(end);
    Diagnostic {
        path: path.to_owned(),
        rule: BOTICA_VERSION_ALIGNED.to_owned(),
        severity,
        start,
        end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message,
    }
}
