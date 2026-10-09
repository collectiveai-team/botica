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
}
