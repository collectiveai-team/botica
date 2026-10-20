//! `lockfile-committed`: a manifest has the lockfile of one package manager, next to it or above.

use botica_core::lockfiles::{check, Source};
use botica_core::Severity;

fn reported(paths: &[&str], manifests: &[(&str, &str)]) -> Vec<String> {
    let paths: Vec<String> = paths.iter().map(|path| (*path).to_owned()).collect();
    let sources: Vec<Source<'_>> = manifests
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    check(&paths, &sources, Severity::Error)
        .into_iter()
        .map(|finding| finding.path)
        .collect()
}

const PROJECT: &str = "[project]\nname = \"a\"\n";

#[test]
fn a_lockfile_at_the_root_of_the_workspace_pins_its_members() {
    let paths = ["uv.lock", "pyproject.toml", "packages/a/pyproject.toml"];
    let manifests = [
        ("pyproject.toml", PROJECT),
        ("packages/a/pyproject.toml", PROJECT),
    ];
    assert!(reported(&paths, &manifests).is_empty());
}

#[test]
fn a_manifest_with_no_lockfile_above_it_is_reported() {
    let paths = ["packages/a/pyproject.toml", "packages/b/uv.lock"];
    let manifests = [("packages/a/pyproject.toml", PROJECT)];
    assert_eq!(reported(&paths, &manifests), ["packages/a/pyproject.toml"]);
}

#[test]
fn two_files_of_one_manager_are_not_two_managers() {
    let manifest = "{\"dependencies\": {\"a\": \"1\"}}";
    let paths = ["package.json", "bun.lock", "bun.lockb"];
    assert!(reported(&paths, &[("package.json", manifest)]).is_empty());
}

#[test]
fn a_manifest_that_pins_nothing_needs_no_lockfile() {
    let manifests = [
        ("go.mod", "module a\n\ngo 1.25\n// require nothing\n"),
        ("package.json", "{\"name\": \"a\", \"dependencies\": {}}"),
        ("pyproject.toml", "[tool.ruff]\nline-length = 100\n"),
    ];
    assert!(reported(&["go.mod", "package.json", "pyproject.toml"], &manifests).is_empty());
}

#[test]
fn the_package_manager_named_in_package_json_decides_the_lockfile() {
    let manifest =
        "{\"packageManager\": \"pnpm@10.18.0+sha512.abc\", \"devDependencies\": {\"a\": \"1\"}}";
    assert!(reported(
        &["package.json", "pnpm-lock.yaml"],
        &[("package.json", manifest)]
    )
    .is_empty());
    assert_eq!(
        reported(
            &["package.json", "package-lock.json"],
            &[("package.json", manifest)]
        ),
        ["package.json"]
    );
}
