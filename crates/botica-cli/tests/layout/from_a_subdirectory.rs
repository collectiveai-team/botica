use super::*;

/// A project whose root holds `botica.toml` and two projects.
fn monorepo(name: &str) -> PathBuf {
    let dir = project(name);
    write(&dir, "botica.toml", "");
    backend(&dir);
    write(&dir, "backend/amarc/test_core.py", "x = 1\n");
    write(&dir, "backend/amarc/core/test_more.py", "x = 1\n");
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/src/weights.pt", "w\n");
    dir
}

fn lint_from(dir: &Path, below: &str) -> String {
    stdout(&command(&dir.join(below), &["lint", ".", "--no-astgrep"]))
}

#[test]
fn from_inside_a_project_the_code_is_found_below_the_directory() {
    let dir = monorepo("from-project");

    let text = lint_from(&dir, "backend");

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["amarc/core/test_more.py", "amarc/test_core.py"],
        "{text}"
    );
}

#[test]
fn from_inside_the_package_everything_is_inside_it() {
    let dir = monorepo("from-package");

    let text = lint_from(&dir, "backend/amarc");

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["core/test_more.py", "test_core.py"],
        "{text}"
    );
}

#[test]
fn from_deeper_inside_the_package_the_whole_tree_is_still_inside_it() {
    let dir = monorepo("from-deep");

    let text = lint_from(&dir, "backend/amarc/core");

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["test_more.py"],
        "{text}"
    );
}

#[test]
fn from_a_directory_outside_every_code_directory_nothing_is_reported() {
    let dir = monorepo("from-outside");
    write(&dir, "docs/test_a.py", "x = 1\n");
    write(&dir, "docs/rows.csv", "a,b\n");

    let text = lint_from(&dir, "docs");

    assert_eq!(reported(&text, "no-tests-in-package"), nothing(), "{text}");
    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        nothing(),
        "{text}"
    );
}

#[test]
fn from_another_project_its_own_code_is_found() {
    let dir = monorepo("from-frontend");

    let text = lint_from(&dir, "frontend");

    assert_eq!(
        reported(&text, "no-artifacts-in-package"),
        ["src/weights.pt"],
        "{text}"
    );
    assert_eq!(reported(&text, "no-tests-in-package"), nothing(), "{text}");
}
