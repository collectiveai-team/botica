use super::*;

/// A project that holds a catch-all module in `tests/`, one in `pkg/tests/` and one in `pkg/`.
fn catch_alls(name: &str) -> PathBuf {
    let dir = project(name);
    write(&dir, "botica.toml", "");
    write(&dir, "tests/utils.py", "x = 1\n");
    write(&dir, "pkg/tests/helpers.py", "x = 1\n");
    write(&dir, "pkg/utils.py", "x = 1\n");
    dir
}

fn utils_from(dir: &Path, below: &str) -> Vec<String> {
    let text = stdout(&command(
        &dir.join(below),
        &["lint", ".", "--no-astgrep", "--select", "no-utils"],
    ));
    reported(&text, "no-utils")
}

#[test]
fn from_inside_tests_the_exemption_of_tests_still_holds() {
    let dir = catch_alls("utils-from-tests");

    assert_eq!(utils_from(&dir, "tests"), nothing());
    assert_eq!(utils_from(&dir, "pkg/tests"), nothing());
}

#[test]
fn from_a_directory_that_is_not_tests_a_catch_all_is_still_reported() {
    let dir = catch_alls("utils-from-pkg");

    assert_eq!(utils_from(&dir, "pkg"), ["utils.py"]);
    assert_eq!(utils_from(&dir, "."), ["pkg/utils.py"]);
}
