use crate::issue::CatalogError;
use crate::load::{Catalog, SourceFile};
use crate::references::References;

include!(concat!(env!("OUT_DIR"), "/embedded_files.rs"));

impl Catalog {
    pub fn embedded() -> Result<Self, CatalogError> {
        let files: Vec<SourceFile> = EMBEDDED_FILES
            .iter()
            .map(|(path, text)| SourceFile {
                path: (*path).to_owned(),
                text: (*text).to_owned(),
            })
            .collect();
        Self::from_files(&files)
    }
}

impl References {
    /// The topics that the binary carries: `lint/references/`.
    pub fn embedded() -> Result<Self, CatalogError> {
        let files: Vec<SourceFile> = EMBEDDED_REFERENCES
            .iter()
            .map(|(path, text)| SourceFile {
                path: (*path).to_owned(),
                text: (*text).to_owned(),
            })
            .collect();
        Self::from_builtin_files(&files)
    }
}
