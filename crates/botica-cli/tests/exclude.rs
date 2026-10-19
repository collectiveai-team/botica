mod common;

use common::{botica, project_in, stdout, write};
use std::path::PathBuf;

/// `utils.py` breaks `no-utils` at the top and under `vendor/`.
fn project(name: &str, exclude: &str) -> PathBuf {
    let dir = project_in(
        "exclude",
        name,
        Some(&format!("extend-exclude = [{exclude}]\n")),
    );
    write(&dir, "utils.py", "x = 1\n");
    write(&dir, "vendor/utils.py", "x = 1\n");
    dir
}

fn reported(args: &[&str], dir: &std::path::Path) -> Vec<String> {
    let output = botica(dir, args);
    stdout(&output)
        .lines()
        .filter_map(|line| line.split(':').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_walk_skips_what_extend_exclude_names() {
    let dir = project("walk", "\"vendor\"");
    assert_eq!(reported(&[], &dir), ["utils.py"]);
    assert_eq!(reported(&["."], &dir), ["utils.py"]);
}

#[test]
fn a_glob_with_a_directory_is_anchored_to_the_project_root() {
    let dir = project("anchored", "\"vendor/**\"");
    assert_eq!(reported(&[], &dir), ["utils.py"]);
}

#[test]
fn a_path_given_on_the_command_line_is_checked_even_if_excluded() {
    let dir = project("explicit", "\"vendor\"");
    assert_eq!(reported(&["vendor/utils.py"], &dir), ["vendor/utils.py"]);
    assert_eq!(reported(&["vendor"], &dir), ["vendor/utils.py"]);
}

#[test]
fn force_exclude_respects_the_exclusion_of_paths_given_on_the_command_line() {
    let dir = project("forced", "\"vendor\"");
    assert!(reported(&["--force-exclude", "vendor/utils.py"], &dir).is_empty());
    assert!(reported(&["--force-exclude", "vendor"], &dir).is_empty());
    assert_eq!(
        reported(&["--force-exclude", "vendor/utils.py", "utils.py"], &dir),
        ["utils.py"]
    );
}

#[test]
fn force_exclude_does_not_report_an_excluded_path_that_does_not_exist() {
    let dir = project("forced-missing", "\"gone\"");
    let output = botica(&dir, &["--force-exclude", "gone/x.py"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(stdout(&output).is_empty());
}

#[test]
fn the_exclusion_is_read_from_the_project_root_when_running_from_a_subdirectory() {
    let dir = project("nested", "\"sub/vendor\"");
    write(&dir, "sub/utils.py", "x = 1\n");
    write(&dir, "sub/vendor/utils.py", "x = 1\n");
    let sub = dir.join("sub");
    assert_eq!(reported(&["."], &sub), ["utils.py"]);
    assert!(reported(&["--force-exclude", "vendor/utils.py"], &sub).is_empty());
}
