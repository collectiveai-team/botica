//! Whether two globs can select the same path, decided on the globs and not on sample paths.
//!
//! The globs are the ones of a `.gitignore` as `PathMatcher` reads them. A `[...]` class is read
//! as `?` and braces with too many alternatives as `**`, so a doubt is answered with an overlap:
//! the callers refuse a configuration on an overlap.

/// The most alternatives that braces expand to; a glob with more is read as `**`.
const MAX_ALTERNATIVES: usize = 64;

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

fn expand_braces(glob: &str) -> Vec<String> {
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
        for rest in expand_braces(&format!("{head}{}{tail}", &glob[start..end])) {
            all.push(rest);
            if all.len() > MAX_ALTERNATIVES {
                return vec!["**".to_owned()];
            }
        }
    }
    all
}

fn tokens(segment: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if out.last() != Some(&Token::Star) {
                    out.push(Token::Star);
                }
            }
            '?' => out.push(Token::One),
            '[' => {
                let rest = chars.as_str();
                match rest.find(']') {
                    Some(end) if end > 0 => {
                        chars = rest[end + 1..].chars();
                        out.push(Token::One);
                    }
                    _ => out.push(Token::Literal('[')),
                }
            }
            '\\' => out.extend(chars.next().map(Token::Literal)),
            other => out.push(Token::Literal(other)),
        }
    }
    out
}

/// A glob with no `/` in it matches at every depth, as in a `.gitignore`. A last name with no
/// wildcard may be a directory, so what is inside it is selected too, unless `file` says that the
/// glob names a file or the name has an extension (`README.md`, and not `docs` or `.github`).
fn alternative(glob: &str, file: bool) -> Alternative {
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
    let literal_last = matches!(
        segments.last(),
        Some(Segment::Name(name))
            if name.iter().all(|t| matches!(t, Token::Literal(_)))
                && !name.iter().skip(1).any(|t| *t == Token::Literal('.'))
    );
    if literal_last && !file {
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
        Self::build(text, false)
    }

    /// A glob that names a file or files, as `botica.toml` or `**/AGENTS.md`.
    pub(crate) fn file(text: &str) -> Self {
        Self::build(text, true)
    }

    fn build(text: &str, file: bool) -> Self {
        let expanded = expand_braces(text);
        Self {
            picks_files: expanded
                .iter()
                .any(|glob| !glob.trim_end_matches('/').ends_with("**")),
            alternatives: expanded
                .iter()
                .map(|glob| alternative(glob, file))
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
}
