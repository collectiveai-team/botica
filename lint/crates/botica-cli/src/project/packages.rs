use std::path::Path;

/// Files that mark a directory as a package of some language.
const MARKERS: [&str; 5] = [
    "pyproject.toml",
    "package.json",
    "Cargo.toml",
    "go.mod",
    "__init__.py",
];

/// Directories that sit next to a package by the standard layout and are never one, even with an
/// `__init__.py` in them.
const NOT_PACKAGES: [&str; 2] = ["tests", "test"];

/// The directories of `root` that hold a package: the ones with a marker of any language, and not
/// `src/`. Sorted.
pub fn detect(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !NOT_PACKAGES.contains(&name.as_str()))
        .filter(|name| name != "src")
        .filter(|name| {
            MARKERS
                .iter()
                .any(|marker| root.join(name).join(marker).is_file())
        })
        .collect();
    found.sort();
    found
}
