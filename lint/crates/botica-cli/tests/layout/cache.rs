use super::*;

fn lint_with_cache(dir: &Path) -> String {
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");
    stdout(&command(
        dir,
        &["lint", ".", "--no-astgrep", "--cache-dir", cache],
    ))
}

#[test]
fn the_cache_follows_a_change_in_where_the_code_is() {
    let dir = project("cache-code-dirs");
    write(&dir, "pyproject.toml", PYPROJECT);
    write(&dir, "pkg/data.csv", "a,b\n");

    let before = lint_with_cache(&dir);
    write(&dir, "pkg/__init__.py", "");
    let after = lint_with_cache(&dir);

    assert_eq!(
        reported(&before, "no-artifacts-in-package"),
        nothing(),
        "{before}"
    );
    assert_eq!(
        reported(&after, "no-artifacts-in-package"),
        ["pkg/data.csv"],
        "{after}"
    );
}
