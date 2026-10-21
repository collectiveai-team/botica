//! A unified diff, as `git diff` writes it: which files a change adds or changes, the lines it
//! adds, and the text of a file before the change (ADR 0009).

use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;
use std::fmt;

/// The files that a diff adds or changes, by their path on the new side. A deleted file is not
/// in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    files: BTreeMap<String, FileChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FileChange {
    /// The lines the diff adds, 1-based, on the new side.
    added: Vec<usize>,
    hunks: Vec<Hunk>,
    /// The diff creates the file: it has no text before the change.
    created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Hunk {
    /// The first line of the hunk on the new side; with no line of context and nothing added, the
    /// line after which the removed lines were.
    new_start: usize,
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HunkLine {
    Context(String),
    Added(String),
    /// A removed line, and whether it ended with a newline.
    Removed(String, bool),
}

/// A line of a diff that is not where a diff can have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Malformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} of the diff: {}", self.line, self.message)
    }
}

impl std::error::Error for Malformed {}

/// The hunks of a file do not apply to its text: the diff does not describe that file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub path: String,
    pub line: usize,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the diff does not apply to `{}`: line {} differs",
            self.path, self.line
        )
    }
}

impl std::error::Error for Mismatch {}

impl Changes {
    /// Reads the text of a unified diff. The paths lose the `a/` and `b/` that git puts before
    /// them.
    pub fn parse(text: &str) -> Result<Self, Malformed> {
        Parser::default().run(text)
    }

    /// Adds a file that the diff does not show and that is new as a whole, such as a file git does
    /// not track yet: each of its `lines` is added.
    pub fn add_created(&mut self, path: &str, lines: usize) {
        self.files.insert(
            path.to_owned(),
            FileChange {
                added: (1..=lines).collect(),
                hunks: Vec::new(),
                created: true,
            },
        );
    }

    /// The paths of the files that the diff adds or changes.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Whether the diff creates the file.
    pub fn creates(&self, path: &str) -> bool {
        self.files.get(path).is_some_and(|file| file.created)
    }

    /// Whether a finding stays with this diff: one with no path always, one about a whole file
    /// when the diff adds or changes the file, and one with a range when the range touches a line
    /// the diff adds.
    pub fn keeps(&self, diagnostic: &Diagnostic) -> bool {
        if diagnostic.path.is_empty() {
            return true;
        }
        let Some(file) = self.files.get(&diagnostic.path) else {
            return false;
        };
        if diagnostic.file_level {
            return true;
        }
        let first = diagnostic.line;
        let last = diagnostic.end_line.max(first);
        let from = file.added.partition_point(|&line| line < first);
        file.added.get(from).is_some_and(|&line| line <= last)
    }

    /// The text of the file `path` before the change, from its text after it: the hunks applied in
    /// reverse. `None` when the diff creates the file or does not have it.
    pub fn base(&self, path: &str, text: &str) -> Result<Option<String>, Mismatch> {
        match self.files.get(path) {
            Some(file) if !file.created => reverse(path, &file.hunks, text).map(Some),
            _ => Ok(None),
        }
    }
}

fn reverse(path: &str, hunks: &[Hunk], text: &str) -> Result<String, Mismatch> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut base = String::with_capacity(text.len());
    let mut next = 0;
    for hunk in hunks {
        let starts_at = if hunk
            .lines
            .iter()
            .any(|line| !matches!(line, HunkLine::Removed(..)))
        {
            hunk.new_start.saturating_sub(1)
        } else {
            hunk.new_start
        };
        let mismatch = |index: usize| Mismatch {
            path: path.to_owned(),
            line: index + 1,
        };
        if starts_at < next || starts_at > lines.len() {
            return Err(mismatch(starts_at.min(lines.len())));
        }
        base.extend(lines[next..starts_at].iter().copied());
        next = starts_at;
        for line in &hunk.lines {
            match line {
                HunkLine::Context(expected) | HunkLine::Added(expected) => {
                    let actual = lines.get(next).ok_or_else(|| mismatch(next))?;
                    if actual.strip_suffix('\n').unwrap_or(actual) != expected {
                        return Err(mismatch(next));
                    }
                    if matches!(line, HunkLine::Context(_)) {
                        base.push_str(actual);
                    }
                    next += 1;
                }
                HunkLine::Removed(removed, newline) => {
                    base.push_str(removed);
                    if *newline {
                        base.push('\n');
                    }
                }
            }
        }
    }
    base.extend(lines[next..].iter().copied());
    Ok(base)
}

#[derive(Default)]
struct Parser {
    changes: Changes,
    /// The file the lines being read belong to; `None` for a deleted file, whose lines are skipped.
    current: Option<String>,
    old_is_null: bool,
    /// Lines still due in the hunk being read, on each side.
    old_left: usize,
    new_left: usize,
    new_line: usize,
}

impl Parser {
    fn run(mut self, text: &str) -> Result<Changes, Malformed> {
        // Split on `\n` alone: a file with `\r\n` keeps its `\r` in the lines of its hunks.
        let text = text.strip_suffix('\n').unwrap_or(text);
        for (index, line) in text.split('\n').enumerate() {
            self.line(line).map_err(|message| Malformed {
                line: index + 1,
                message,
            })?;
        }
        Ok(self.changes)
    }

    fn line(&mut self, line: &str) -> Result<(), String> {
        if self.old_left > 0 || self.new_left > 0 {
            return self.hunk_line(line);
        }
        if let Some(rest) = line.strip_prefix("diff --git ") {
            self.current = rest.rsplit_once(" b/").map(|(_, path)| path.to_owned());
            self.old_is_null = false;
            self.register();
        } else if let Some(path) = line.strip_prefix("--- ") {
            self.old_is_null = path == "/dev/null";
        } else if let Some(path) = line.strip_prefix("+++ ") {
            self.current = (path != "/dev/null").then(|| path_of(path, "b/"));
            self.register();
        } else if let Some(path) = line.strip_prefix("rename to ") {
            self.current = Some(unquote(path));
            self.register();
        } else if line.starts_with("new file mode") {
            self.old_is_null = true;
            self.register();
        } else if line.starts_with("deleted file mode") {
            if let Some(path) = self.current.take() {
                self.changes.files.remove(&path);
            }
        } else if let Some(header) = line.strip_prefix("@@ ") {
            self.hunk_header(header)?;
        } else if line.starts_with('\\') {
            self.no_newline();
        }
        Ok(())
    }

    fn register(&mut self) {
        if let Some(path) = &self.current {
            let created = self.old_is_null;
            self.changes.files.entry(path.clone()).or_default().created = created;
        }
    }

    fn hunk_header(&mut self, header: &str) -> Result<(), String> {
        let ranges = header.split(" @@").next().unwrap_or_default();
        let (old, new) = ranges
            .split_once(' ')
            .ok_or_else(|| format!("`@@ {header}` is not the header of a hunk"))?;
        let (_, old_count) = range(old.strip_prefix('-'), header)?;
        let (new_start, new_count) = range(new.strip_prefix('+'), header)?;
        self.old_left = old_count;
        self.new_left = new_count;
        self.new_line = new_start;
        if let Some(file) = self.file() {
            file.hunks.push(Hunk {
                new_start,
                lines: Vec::new(),
            });
        }
        Ok(())
    }

    fn hunk_line(&mut self, line: &str) -> Result<(), String> {
        let (kind, body) = match line.chars().next() {
            Some(kind @ ('+' | '-' | ' ')) => (kind, &line[1..]),
            None => (' ', ""),
            Some('\\') => {
                self.no_newline();
                return Ok(());
            }
            Some(_) => return Err(format!("`{line}` is not a line of a hunk")),
        };
        let new_line = self.new_line;
        let entry = match kind {
            '+' => {
                self.new_left = self.new_left.saturating_sub(1);
                self.new_line += 1;
                HunkLine::Added(body.to_owned())
            }
            '-' => {
                self.old_left = self.old_left.saturating_sub(1);
                HunkLine::Removed(body.to_owned(), true)
            }
            _ => {
                self.old_left = self.old_left.saturating_sub(1);
                self.new_left = self.new_left.saturating_sub(1);
                self.new_line += 1;
                HunkLine::Context(body.to_owned())
            }
        };
        if let Some(file) = self.file() {
            if matches!(entry, HunkLine::Added(_)) {
                file.added.push(new_line);
            }
            if let Some(hunk) = file.hunks.last_mut() {
                hunk.lines.push(entry);
            }
        }
        Ok(())
    }

    /// `\ No newline at end of file`: the line before it ends without a newline.
    fn no_newline(&mut self) {
        let last = self
            .file()
            .and_then(|file| file.hunks.last_mut())
            .and_then(|hunk| hunk.lines.last_mut());
        if let Some(HunkLine::Removed(_, newline)) = last {
            *newline = false;
        }
    }

    fn file(&mut self) -> Option<&mut FileChange> {
        let path = self.current.as_ref()?;
        self.changes.files.get_mut(path)
    }
}

/// `start,count` of a hunk header; a count left out is 1.
fn range(text: Option<&str>, header: &str) -> Result<(usize, usize), String> {
    let malformed = || format!("`@@ {header}` is not the header of a hunk");
    let text = text.ok_or_else(malformed)?;
    let (start, count) = text.split_once(',').unwrap_or((text, "1"));
    let number = |part: &str| part.parse::<usize>().map_err(|_| malformed());
    Ok((number(start)?, number(count)?))
}

/// A path of a `+++` line: unquoted, without the prefix git gives the new side and without the
/// tab and the time some tools add after it.
fn path_of(text: &str, prefix: &str) -> String {
    let text = text.split('\t').next().unwrap_or(text);
    let path = unquote(text);
    path.strip_prefix(prefix)
        .map_or(path.clone(), str::to_owned)
}

/// A path that git wrote between quotes, with its escapes undone.
fn unquote(text: &str) -> String {
    let Some(inner) = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return text.to_owned();
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}
