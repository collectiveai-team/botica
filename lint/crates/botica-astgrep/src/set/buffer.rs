use super::AstGrepSet;
use crate::language::language_of;
use crate::rule::Hit;
use anyhow::Result;
use ast_grep_core::source::Edit;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::AstGrep;
use ast_grep_language::{LanguageExt, SupportLang};
use std::path::{Path, PathBuf};

impl AstGrepSet {
    /// Opens an editable buffer (LSP): the syntax tree is kept and re-used between edits.
    pub fn open(&self, path: &Path, text: &str) -> Option<Buffer> {
        let language = language_of(path)?;
        self.groups.iter().find(|g| g.language == language)?;
        Some(Buffer {
            path: path.to_path_buf(),
            language,
            grep: language.ast_grep(text),
        })
    }

    pub fn check_buffer(&self, buffer: &Buffer) -> Vec<Hit> {
        let Some(group) = self.groups.iter().find(|g| g.language == buffer.language) else {
            return Vec::new();
        };
        let mut active = vec![false; self.rules.len()];
        for &index in &group.rules {
            active[index] = self.rules[index].applies_to(&buffer.path);
        }
        self.run(group, &active, &buffer.grep, None)
    }
}

pub struct Buffer {
    path: PathBuf,
    language: SupportLang,
    grep: AstGrep<StrDoc<SupportLang>>,
}

impl Buffer {
    pub fn text(&self) -> &str {
        &self.grep.root().get_doc().src
    }

    /// Replaces `deleted` bytes at `start` with `inserted`, re-parsing incrementally.
    pub fn edit(&mut self, start: usize, deleted: usize, inserted: &str) -> Result<()> {
        self.grep
            .edit(Edit {
                position: start,
                deleted_length: deleted,
                inserted_text: inserted.as_bytes().to_vec(),
            })
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn language(&self) -> SupportLang {
        self.language
    }
}
