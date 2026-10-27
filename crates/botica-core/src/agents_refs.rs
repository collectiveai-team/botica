//! `agents-md-refs-exist`: each `@<path>` reference of an `AGENTS.md` or a `CLAUDE.md` names a
//! file or a folder of the repository. An agent reads the file that the reference names, and goes
//! on without it when it is missing.

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;

pub const AGENTS_MD_REFS_EXIST: &str = "agents-md-refs-exist";

/// The files of instructions for agents that the rule reads.
const NAMES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// The extensions that make a reference with no `/` a path: `@README.md` is one, `@pytest.fixture`
/// is a decorator and `@latest` a tag.
const EXTENSIONS: [&str; 24] = [
    "md", "mdx", "txt", "rst", "adoc", "json", "jsonc", "toml", "yaml", "yml", "ini", "cfg", "py",
    "ts", "tsx", "js", "mjs", "rs", "go", "sh", "sql", "html", "css", "lock",
];

/// What may come right before the `@` of a reference: an email or a handle in a URL has a letter
/// or a `/` there.
const OPENERS: [char; 6] = ['`', '(', '[', '"', '\'', '<'];

/// A file of instructions for agents, from the root of the project, and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// Whether the rule reads the text of `path`.
pub fn reads(path: &str) -> bool {
    NAMES.contains(&name_of(path))
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// A reference of a file: the path after the `@`, and the range of the `@` and the path in bytes.
#[derive(Debug, PartialEq, Eq)]
pub struct Reference<'a> {
    pub path: &'a str,
    pub start: usize,
    pub end: usize,
}

fn in_path(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '/' | '_' | '-' | '~' | '+')
}

/// The `@<path>` references of `text`, outside its fenced blocks of code. Each one starts a line
/// or follows a space or one of `OPENERS`, and its path has a `/` or ends in one of `EXTENSIONS`.
/// A path that a `@` follows (`@scope/pkg@1.0`) is a package with its version.
pub fn references(text: &str) -> Vec<Reference<'_>> {
    let mut found = Vec::new();
    let mut fence: Option<&str> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let marker = line.trim_start();
        let opens = ["```", "~~~"].into_iter().find(|f| marker.starts_with(f));
        match (fence, opens) {
            (None, Some(f)) => fence = Some(f),
            (Some(open), Some(f)) if open == f => fence = None,
            _ => {}
        }
        if fence.is_none() && opens.is_none() {
            found.extend(
                line.match_indices('@')
                    .filter_map(|(at, _)| reference_at(line, at, start)),
            );
        }
    }
    found
}

/// The reference that the `@` at byte `at` of `line` opens, if it is one. `start` is where the line
/// starts in its file.
fn reference_at(line: &str, at: usize, start: usize) -> Option<Reference<'_>> {
    let before = line[..at].chars().next_back();
    if before.is_some_and(|c| !c.is_whitespace() && !OPENERS.contains(&c)) {
        return None;
    }
    let rest = &line[at + 1..];
    let length = rest.find(|c: char| !in_path(c)).unwrap_or(rest.len());
    let path = rest[..length].trim_end_matches(['.', ',', ':', ';']);
    if rest[path.len()..].starts_with('@') || !is_path(path) {
        return None;
    }
    Some(Reference {
        path,
        start: start + at,
        end: start + at + 1 + path.len(),
    })
}

fn is_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with(['/', '~']) {
        return false;
    }
    path.contains('/')
        || path
            .rsplit_once('.')
            .is_some_and(|(stem, extension)| !stem.is_empty() && EXTENSIONS.contains(&extension))
}

/// The folders of `path` joined to `dir`, with `.` and `..` resolved; `None` when `..` leaves the
/// root of the project.
fn joined(dir: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// Whether a reference with a `/`, no extension of `EXTENSIONS` and a first folder that the
/// project does not have is something else than a path, such as the npm package `@types/node`.
fn foreign(path: &str, candidates: &[String], exists: &dyn Fn(&str) -> bool) -> bool {
    let has_extension = name_of(path.trim_end_matches('/'))
        .rsplit_once('.')
        .is_some_and(|(_, extension)| EXTENSIONS.contains(&extension));
    let first = path.split('/').find(|part| !matches!(*part, "" | "."));
    !has_extension
        && first.is_none_or(|first| {
            first != ".."
                && candidates
                    .iter()
                    .filter_map(|base| joined(base, first))
                    .all(|folder| !exists(&folder))
        })
}

/// The references of `sources` that name nothing in the project. `exists` says whether a path from
/// the root of the project is a file or a folder on disk, and `ignored` whether `.gitignore` leaves
/// it out: a tool writes such a path (`.agents/skills/…`), and a fresh clone lacks it. A reference
/// resolves from the folder of its file, as Claude Code reads it, or from the root of the project.
pub fn check(
    sources: &[Source<'_>],
    exists: &dyn Fn(&str) -> bool,
    ignored: &dyn Fn(&str) -> bool,
    severity: Severity,
) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for source in sources {
        let document = Document::new(source.path, source.text);
        let bases = [dir_of(source.path).to_owned(), String::new()];
        for reference in references(source.text) {
            let targets: Vec<String> = bases
                .iter()
                .filter_map(|base| joined(base, reference.path))
                .collect();
            if targets.is_empty()
                || targets
                    .iter()
                    .any(|target| exists(target) || ignored(target))
                || foreign(reference.path, &bases, exists)
            {
                continue;
            }
            let (line, column) = document.position(reference.start);
            let (end_line, end_column) = document.position(reference.end);
            found.push(Diagnostic {
                path: source.path.to_owned(),
                rule: AGENTS_MD_REFS_EXIST.to_owned(),
                severity,
                start: reference.start,
                end: reference.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                measure: None,
                message: format!(
                    "`@{}` names no file or folder of the repository: fix the path, or remove \
                     the reference",
                    reference.path
                ),
            });
        }
    }
    found
}
