//! Whether two globs can select the same path, decided on the globs and not on sample paths.
//!
//! The globs are the ones of a `.gitignore` as `PathMatcher` reads them. The callers refuse a
//! configuration on an overlap, so the module reports an overlap when it cannot tell. A glob with
//! a character class, an escape or too many braces is read as `**`, and a last name that may be a
//! folder selects what is inside it.

/// A glob with more groups of braces, or more commas in them, is read as `**`: the expansion
/// grows with the product of the alternatives of the groups.
const MAX_BRACE_GROUPS: usize = 4;
const MAX_BRACE_COMMAS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(char),
    One,
    Star,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    AnyDepth,
    Name(Vec<Token>),
}

type Alternative = Vec<Segment>;

#[derive(Debug, Clone)]
pub(crate) struct Glob {
    alternatives: Vec<Alternative>,
    /// Whether some alternative, as written, ends with a name and not with `**`.
    picks_files: bool,
}

/// The globs that the groups of braces of `glob` stand for: `{a,b}/x` gives `a/x` and `b/x`.
pub fn expand_braces(glob: &str) -> Vec<String> {
    let Some(open) = glob.find('{') else {
        return vec![glob.to_owned()];
    };
    let mut depth = 0usize;
    let mut close = None;
    let mut cuts = Vec::new();
    for (offset, c) in glob[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + offset);
                    break;
                }
            }
            ',' if depth == 1 => cuts.push(open + offset),
            _ => {}
        }
    }
    let Some(close) = close else {
        return vec![glob.to_owned()];
    };
    let (head, tail) = (&glob[..open], &glob[close + 1..]);
    let mut starts = vec![open + 1];
    starts.extend(cuts.iter().map(|cut| cut + 1));
    let mut ends = cuts;
    ends.push(close);
    let mut all = Vec::new();
    for (start, end) in starts.into_iter().zip(ends) {
        all.extend(expand_braces(&format!("{head}{}{tail}", &glob[start..end])));
    }
    all
}

/// Whether the glob has something that this module does not follow exactly.
fn too_complex(glob: &str) -> bool {
    glob.contains(['[', '\\'])
        || glob.matches('{').count() > MAX_BRACE_GROUPS
        || glob.matches(',').count() > MAX_BRACE_COMMAS
}

fn tokens(segment: &str) -> Vec<Token> {
    let mut out = Vec::new();
    for c in segment.chars() {
        match c {
            '*' => {
                if out.last() != Some(&Token::Star) {
                    out.push(Token::Star);
                }
            }
            '?' => out.push(Token::One),
            other => out.push(Token::Literal(other)),
        }
    }
    out
}

/// A name that ends in a literal extension (`uv.lock`, `*.md`, and not `docs`, `.github`, `s*` or
/// `*.lock*`): it is read as a file. A folder named `a.b` is a case that this reading misses.
fn names_a_file(name: &[Token]) -> bool {
    let dot = name.iter().rposition(|t| *t == Token::Literal('.'));
    dot.is_some_and(|at| at > 0 && at + 1 < name.len())
        && name
            .iter()
            .rev()
            .take_while(|t| **t != Token::Literal('.'))
            .all(|t| matches!(t, Token::Literal(_)))
}

/// How the last name of a glob is read: as a file, as a folder (what is inside it is selected
/// too) or, for `Guess`, as a folder unless the name looks like a file.
#[derive(Clone, Copy)]
enum Reading {
    Guess,
    File,
    Folder,
}

/// A glob with no `/` in it matches at every depth, as in a `.gitignore`.
fn alternative(glob: &str, reading: Reading) -> Alternative {
    let reading = if glob.ends_with('/') {
        Reading::Folder
    } else {
        reading
    };
    let glob = glob.trim_end_matches('/');
    let anchored = glob.contains('/');
    let mut segments: Alternative = glob
        .split('/')
        .filter(|part| !part.is_empty())
        .map(|part| {
            if part == "**" {
                Segment::AnyDepth
            } else {
                Segment::Name(tokens(part))
            }
        })
        .collect();
    if !anchored {
        segments.insert(0, Segment::AnyDepth);
    }
    let ends_with_a_file =
        matches!(segments.last(), Some(Segment::Name(name)) if names_a_file(name));
    let ends_with_a_name = matches!(segments.last(), Some(Segment::Name(_)));
    let folder = match reading {
        Reading::Folder => true,
        Reading::File => false,
        Reading::Guess => !ends_with_a_file,
    };
    if ends_with_a_name && folder {
        segments.push(Segment::AnyDepth);
    }
    segments
}

/// An element of a glob that can stand for several elements of another: `*` in a name, `**` in
/// a path.
trait Element {
    fn wild(&self) -> bool;
    /// Whether both can stand for the same one element; a wild element is handled apart.
    fn same(&self, other: &Self) -> bool;
}

impl Element for Token {
    fn wild(&self) -> bool {
        *self == Self::Star
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Literal(x), Self::Literal(y)) => x == y,
            (Self::Star, _) | (_, Self::Star) => false,
            _ => true,
        }
    }
}

impl Element for Segment {
    fn wild(&self) -> bool {
        *self == Self::AnyDepth
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Name(x), Self::Name(y)) => intersect(x, y),
            _ => false,
        }
    }
}

type Table = Vec<Vec<bool>>;

/// A wild element at `i` of `a` takes nothing or one more element of `b`.
fn wild_step<T: Element>(a: &[T], b_len: usize, table: &Table, i: usize, j: usize) -> bool {
    i < a.len() && a[i].wild() && (table[i + 1][j] || (j < b_len && table[i][j + 1]))
}

/// A wild element at `j` of `b` takes nothing or one more element of `a`.
fn wild_step_of_b<T: Element>(b: &[T], a_len: usize, table: &Table, i: usize, j: usize) -> bool {
    j < b.len() && b[j].wild() && (table[i][j + 1] || (i < a_len && table[i + 1][j]))
}

fn both_step<T: Element>(a: &[T], b: &[T], table: &Table, i: usize, j: usize) -> bool {
    match (a.get(i), b.get(j)) {
        (Some(x), Some(y)) => x.same(y) && table[i + 1][j + 1],
        _ => false,
    }
}

/// Whether some sequence of elements is selected by both `a` and `b`.
fn intersect<T: Element>(a: &[T], b: &[T]) -> bool {
    let (n, m) = (a.len(), b.len());
    let mut table: Table = vec![vec![false; m + 1]; n + 1];
    table[n][m] = true;
    for i in (0..=n).rev() {
        for j in (0..=m).rev() {
            if i < n || j < m {
                table[i][j] = wild_step(a, m, &table, i, j)
                    || wild_step_of_b(b, n, &table, i, j)
                    || both_step(a, b, &table, i, j);
            }
        }
    }
    table[0][0]
}

impl Glob {
    /// A glob of an entry, which may name a directory and then selects what is inside it.
    pub(crate) fn parse(text: &str) -> Self {
        Self::build(text, Reading::Guess)
    }

    /// A glob of an entry whose last name is a folder, whatever the name looks like: `*.github`
    /// may be the folder `.github`.
    pub(crate) fn folder(text: &str) -> Self {
        let last = text
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(text);
        if last.contains(['*', '?']) {
            Self::build(text, Reading::Folder)
        } else {
            Self::build(text, Reading::Guess)
        }
    }

    /// A glob that names a file or files, as `botica.toml` or `**/AGENTS.md`.
    pub(crate) fn file(text: &str) -> Self {
        Self::build(text, Reading::File)
    }

    fn build(text: &str, reading: Reading) -> Self {
        if too_complex(text) {
            return Self {
                alternatives: vec![vec![Segment::AnyDepth]],
                picks_files: true,
            };
        }
        let expanded = expand_braces(text);
        Self {
            picks_files: expanded.iter().any(|glob| !glob.ends_with("**")),
            alternatives: expanded
                .iter()
                .map(|glob| alternative(glob, reading))
                .collect(),
        }
    }

    /// Whether some path exists that both globs select.
    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        self.alternatives
            .iter()
            .any(|a| other.alternatives.iter().any(|b| intersect(a, b)))
    }

    /// Whether the glob, as written, ends with a name and not with `**`: it picks files by their
    /// names.
    pub(crate) fn picks_files(&self) -> bool {
        self.picks_files
    }

    /// Whether a name of the glob, between two `/`, can be the name `forced` and spells part of it
    /// out: `docs/.claude/**` and `docs/.cl*/**` name the folder `.claude`, and `docs/**` and
    /// `crates/*/tests/**` do not, since a `*` alone says nothing about the name.
    pub(crate) fn names(&self, forced: &str) -> bool {
        let forced = tokens(forced);
        self.alternatives.iter().any(|a| {
            a.iter().any(|segment| match segment {
                Segment::Name(name) => {
                    name.iter().any(|t| matches!(t, Token::Literal(_))) && intersect(name, &forced)
                }
                Segment::AnyDepth => false,
            })
        })
    }
}
