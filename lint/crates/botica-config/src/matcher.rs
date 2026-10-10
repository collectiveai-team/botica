use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Debug, Clone)]
pub struct PathMatcher {
    inner: Gitignore,
    /// For each pattern that selects paths, the directories it starts with when it is anchored
    /// to the root and begins with literal names; empty when it can match at any depth.
    starts: Vec<Vec<String>>,
}

/// Characters that make a path segment of a pattern more than a literal name.
const GLOB_CHARACTERS: [char; 5] = ['*', '?', '[', '{', '\\'];

/// The literal directories a pattern is anchored under: `generated/api/*.py` gives
/// `["generated", "api"]`, `*.gen.py` and `**/x` give none. `None` for a pattern that selects
/// nothing by itself (a blank line, a comment, a negation).
fn anchored_under(pattern: &str) -> Option<Vec<String>> {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern.starts_with('#') || pattern.starts_with('!') {
        return None;
    }
    let pattern = pattern.strip_suffix('/').unwrap_or(pattern);
    let anchored = pattern.contains('/');
    let pattern = pattern.strip_prefix('/').unwrap_or(pattern);
    if !anchored {
        return Some(Vec::new());
    }
    Some(
        pattern
            .split('/')
            .take_while(|segment| !segment.contains(GLOB_CHARACTERS))
            .map(str::to_owned)
            .collect(),
    )
}

impl PathMatcher {
    pub fn new(patterns: &[String]) -> Result<Self, Vec<(usize, String)>> {
        let mut builder = GitignoreBuilder::new("");
        let mut errors = Vec::new();
        for (index, pattern) in patterns.iter().enumerate() {
            if let Err(error) = builder.add_line(None, pattern) {
                errors.push((index, error.to_string()));
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let inner = builder
            .build()
            .map_err(|error| vec![(0, error.to_string())])?;
        let starts = patterns
            .iter()
            .filter_map(|pattern| anchored_under(pattern))
            .collect();
        Ok(Self { inner, starts })
    }

    pub fn check(pattern: &str) -> Result<(), String> {
        GitignoreBuilder::new("")
            .add_line(None, pattern)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn is_match(&self, path: &str) -> bool {
        self.inner
            .matched_path_or_any_parents(path, false)
            .is_ignore()
    }

    /// Whether this one entry matches, without looking at its parents: for a walk, which prunes a
    /// directory as soon as it matches and so never reaches what is inside.
    pub fn is_match_entry(&self, path: &str, is_dir: bool) -> bool {
        self.inner.matched(path, is_dir).is_ignore()
    }

    /// Whether some path under the directory `dir` could match, so a walk looking for matches
    /// knows which directories are worth entering. It may say yes when nothing matches; it never
    /// says no when something does.
    pub fn may_match_below(&self, dir: &str) -> bool {
        let names: Vec<&str> = dir.split('/').filter(|name| !name.is_empty()).collect();
        self.starts.iter().any(|start| {
            names
                .iter()
                .zip(start)
                .all(|(name, literal)| *name == literal)
        })
    }
}
