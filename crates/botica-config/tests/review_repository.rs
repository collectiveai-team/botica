//! The review areas that this repository declares in `[tool.botica.review]` of its `pyproject.toml`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use botica_config::{load_file, AreaName};
use common::embedded;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The files that git tracks, from the root of the checkout: untracked and ignored files of a
/// working copy do not change the answer.
fn tracked_files() -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(checkout())
        .output()
        .expect("run git");
    assert!(output.status.success(), "git ls-files failed");
    String::from_utf8(output.stdout)
        .expect("paths are UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_areas_of_this_repository_are_valid_and_every_glob_matches_a_tracked_file() {
    let loaded = load_file(&checkout().join("pyproject.toml")).expect("pyproject.toml loads");
    loaded
        .config
        .validate(&embedded())
        .unwrap_or_else(|error| panic!("the review table is invalid:\n{error}"));
    let all = tracked_files();
    let all: Vec<&str> = all.iter().map(String::as_str).collect();

    let unmatched = loaded.config.review.unmatched_paths(&all);

    assert!(unmatched.is_empty(), "no file matches: {unmatched:?}");
}

#[test]
fn a_documentation_file_is_periphery_and_a_workflow_is_platform() {
    let loaded = load_file(&checkout().join("pyproject.toml")).expect("pyproject.toml loads");
    let areas = &loaded.config.review.areas;
    let name_of = |path: &str| {
        areas
            .iter()
            .filter(|area| {
                area.paths.iter().any(|glob| {
                    botica_config::PathMatcher::new(std::slice::from_ref(glob))
                        .expect("valid glob")
                        .is_match(path)
                })
            })
            .map(|area| area.name)
            .collect::<Vec<_>>()
    };

    assert_eq!(name_of("website/docs/guide/lint.md"), [AreaName::Periphery]);
    assert_eq!(name_of(".github/workflows/ci.yml"), [AreaName::Platform]);
    assert_eq!(
        name_of("catalog/recipes/files/github-go/workflows/go.yml"),
        [AreaName::Critical, AreaName::Core]
    );
    assert_eq!(name_of("crates/botica-config/src/lib.rs"), [AreaName::Core]);
}
