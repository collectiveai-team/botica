#[path = "../common/mod.rs"]
mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::{Path, PathBuf};

fn project(name: &str) -> PathBuf {
    let dir = project_in("layout", name, None);
    std::fs::canonicalize(dir).expect("the project exists")
}

/// The paths the run reports for `rule`, in the order of the output.
fn reported(text: &str, rule: &str) -> Vec<String> {
    let tag = format!("[{rule}]");
    text.lines()
        .filter(|line| line.contains(&tag))
        .filter_map(|line| line.split(':').next())
        .map(str::to_owned)
        .collect()
}

fn lint(dir: &Path) -> String {
    stdout(&command(dir, &["lint", ".", "--no-astgrep"]))
}

fn nothing() -> Vec<String> {
    Vec::new()
}

const PYPROJECT: &str = "[project]\nname = \"backend\"\n";

/// A Python project `backend/` whose package is `backend/amarc/`.
fn backend(dir: &Path) {
    write(dir, "backend/pyproject.toml", PYPROJECT);
    write(dir, "backend/amarc/__init__.py", "");
}

mod artifacts;
mod cache;
mod configuration;
mod ecosystems;
mod from_a_subdirectory;
mod no_utils_below_the_root;
mod notebooks;
mod tests_in_package;
