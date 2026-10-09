use ignore::gitignore::{Gitignore, GitignoreBuilder};

#[derive(Debug, Clone)]
pub struct PathMatcher {
    inner: Gitignore,
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
        Ok(Self { inner })
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
}
