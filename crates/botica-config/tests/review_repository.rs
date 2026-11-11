//! The review areas that this repository declares in `[tool.botica.review]` of its `pyproject.toml`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use botica_config::{load_file, AreaName};
use common::embedded;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The files of the checkout from its root, without what git ignores and the folders of tools.
fn files(root: &Path, dir: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read the directory").flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(
            name.as_str(),
            ".git" | "target" | ".worktrees" | ".tmp" | ".scratch" | ".venv" | "node_modules"
        ) {
            continue;
        }
        if path.is_dir() {
            files(root, &path, found);
        } else if let Ok(relative) = path.strip_prefix(root) {
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn the_areas_of_this_repository_are_valid_and_every_glob_matches_a_file() {
    let loaded = load_file(&checkout().join("pyproject.toml")).expect("pyproject.toml loads");
    loaded
        .config
        .validate(&embedded())
        .unwrap_or_else(|error| panic!("the review table is invalid:\n{error}"));
    let mut all = Vec::new();
    files(&checkout(), &checkout(), &mut all);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();

    let unmatched = loaded.config.review.unmatched_paths(&all);

    assert!(unmatched.is_empty(), "no file matches: {unmatched:?}");
}

#[test]
fn this_repository_declares_a_periphery_so_that_a_change_can_reach_the_low_tier() {
    let loaded = load_file(&checkout().join("pyproject.toml")).expect("pyproject.toml loads");

    let names: Vec<AreaName> = loaded.config.review.areas.iter().map(|a| a.name).collect();

    for name in [
        AreaName::Critical,
        AreaName::Platform,
        AreaName::Core,
        AreaName::Periphery,
    ] {
        assert!(names.contains(&name), "{name:?}");
    }
}
