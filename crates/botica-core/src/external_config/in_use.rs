//! Whether a project uses the tool of an external implementation: an external rule asks nothing
//! of a project that does not.

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::extension_run::extensions_of;

/// Whether the project, whose files from its root are `files`, uses the tool of an implementation
/// for `languages` that names the files of its tool in `applies_when`. With `applies_when`, a file
/// that one of its globs matches decides. Without it, a file of one of `languages` decides; `*`, or
/// a language whose files botica does not know, counts as used, as before `applies-when`.
pub fn in_use(languages: &[String], applies_when: &[String], files: &[String]) -> bool {
    if !applies_when.is_empty() {
        let Some(globs) = globs(applies_when) else {
            return true;
        };
        return files.iter().any(|file| globs.is_match(file));
    }
    languages.iter().any(|language| {
        let extensions = extensions(language);
        extensions.is_empty()
            || files.iter().any(|file| {
                file.rsplit_once('.')
                    .is_some_and(|(_, extension)| extensions.contains(&extension))
            })
    })
}

/// The globs of `applies-when`, from the root of the project: `*` stays inside a folder and `**/`
/// matches any folders, the root too. `None` for a glob that does not compile, which the catalog
/// rejects when it loads.
fn globs(patterns: &[String]) -> Option<GlobSet> {
    let mut set = GlobSetBuilder::new();
    for pattern in patterns {
        set.add(
            GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .ok()?,
        );
    }
    set.build().ok()
}

/// The extensions of the files of `language`. `tsx` is a language of the catalog whose files
/// botica reads as `typescript`.
fn extensions(language: &str) -> &'static [&'static str] {
    match language {
        "tsx" => &["tsx"],
        other => extensions_of(other),
    }
}
