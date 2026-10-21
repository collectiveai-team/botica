//! `lockfile-committed` in `botica lint`: the run reads the manifests and lockfiles of the whole
//! project, as the walk reaches them.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTED: &str = "select = [\"lockfile-committed\"]\npreview = true\n";

#[test]
fn a_lockfile_that_gitignore_leaves_out_counts_as_missing() {
    let dir = project_in("lockfile-committed", "ignored", Some(SELECTED));
    write(&dir, "pyproject.toml", "[project]\nname = \"a\"\n");
    write(&dir, "uv.lock", "version = 1\n");
    write(&dir, ".gitignore", "uv.lock\n");

    let output = botica(&dir, &["pyproject.toml"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("pyproject.toml:1:1: error [lockfile-committed] No lockfile pins"),
        "{text}"
    );
}

#[test]
fn a_committed_lockfile_or_a_manifest_in_extend_ignores_is_no_finding() {
    let dir = project_in("lockfile-committed", "committed", Some(SELECTED));
    write(&dir, "pyproject.toml", "[project]\nname = \"a\"\n");
    write(&dir, "uv.lock", "version = 1\n");
    assert!(!stdout(&botica(&dir, &[])).contains("lockfile-committed"));

    let config = format!("{SELECTED}[rules.lockfile-committed]\nextend-ignores = [\"tools/**\"]\n");
    let dir = project_in("lockfile-committed", "ignored-path", Some(&config));
    write(&dir, "tools/pyproject.toml", "[project]\nname = \"t\"\n");
    assert!(!stdout(&botica(&dir, &[])).contains("lockfile-committed"));
}
