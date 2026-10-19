use crate::issue::CatalogError;
use crate::load::{Catalog, SourceFile};
use crate::recipes::Recipes;
use crate::references::References;

include!(concat!(env!("OUT_DIR"), "/embedded_files.rs"));

impl Catalog {
    pub fn embedded() -> Result<Self, CatalogError> {
        Self::from_files(&source_files(EMBEDDED_FILES))
    }
}

/// The `(path, text)` pairs that the build embeds, as files.
fn source_files(embedded: &[(&str, &str)]) -> Vec<SourceFile> {
    embedded
        .iter()
        .map(|(path, text)| SourceFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect()
}

impl References {
    /// The topics that the binary carries: `lint/references/`.
    pub fn embedded() -> Result<Self, CatalogError> {
        Self::from_builtin_files(&source_files(EMBEDDED_REFERENCES))
    }
}

impl Recipes {
    /// The recipes that the binary carries: `lint/recipes/`.
    pub fn embedded() -> Result<Self, CatalogError> {
        Self::from_builtin_files(&source_files(EMBEDDED_RECIPES))
    }
}
