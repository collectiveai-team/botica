//! A file that an editor holds open: its path, its text and, when a pattern rule reads its
//! language, the syntax tree that an edit parses again from where it changed.

use botica_astgrep::Buffer as Tree;

pub struct Buffer {
    pub(crate) path: String,
    text: String,
    pub(crate) tree: Option<Tree>,
}

impl Buffer {
    /// A buffer with no syntax tree: the pattern rules parse its text each time.
    pub fn new(path: &str, text: &str) -> Self {
        Self::with_tree(path, text, None)
    }

    pub(crate) fn with_tree(path: &str, text: &str, tree: Option<Tree>) -> Self {
        Self {
            path: path.to_owned(),
            text: text.to_owned(),
            tree,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the buffer keeps a syntax tree between edits.
    pub const fn is_parsed(&self) -> bool {
        self.tree.is_some()
    }

    /// Replaces the bytes from `start` to `end` with `inserted`. A bound past the end of the text
    /// is its end, and a bound inside a character is the start of that character.
    pub fn edit(&mut self, start: usize, end: usize, inserted: &str) {
        let end = boundary(&self.text, end);
        let start = boundary(&self.text, start.min(end));
        self.text.replace_range(start..end, inserted);
        let parsed = self
            .tree
            .as_mut()
            .is_some_and(|tree| tree.edit(start, end - start, inserted).is_ok());
        if !parsed {
            self.tree = None;
        }
    }

    /// Replaces the whole text.
    pub fn replace(&mut self, text: &str) {
        self.edit(0, self.text.len(), text);
    }
}

fn boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
