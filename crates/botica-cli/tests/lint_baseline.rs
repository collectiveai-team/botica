//! The baseline of known findings: `botica lint --write-baseline` writes the findings that carry a
//! measure, and `botica lint` leaves them out while their value does not grow (ADR 0011).

mod common;

use std::fs;
use std::path::PathBuf;

use common::{botica, project_in, stderr, stdout, write};

const CONFIG: &str = "preview = true\nextend-select = [\"cognitive-complexity\"]\n\
baseline = \"botica-baseline.toml\"\n\n[rules.cognitive-complexity]\nmax = 1\n";

/// `nested` has a cognitive complexity of 3.
const NESTED: &str = "def nested(xs):\n    for x in xs:\n        if x:\n            pass\n";

/// `nested` has a cognitive complexity of 6 here.
const DEEPER: &str =
    "def nested(xs):\n    for x in xs:\n        if x:\n            if x > 1:\n                pass\n";

fn project(name: &str) -> PathBuf {
    let dir = project_in("lint-baseline", name, Some(CONFIG));
    write(&dir, "src/a.py", NESTED);
    dir
}

fn write_baseline(dir: &std::path::Path) {
    let output = botica(dir, &["--write-baseline"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn write_baseline_lists_each_finding_with_its_path_from_the_root_its_function_and_its_value() {
    let dir = project("writes");

    let output = botica(&dir, &["--write-baseline"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("1 known findings written to"),
        "{}",
        stderr(&output)
    );
    let text = fs::read_to_string(dir.join("botica-baseline.toml")).expect("the baseline");
    assert!(
        text.contains(
            "[[finding]]\nrule = \"cognitive-complexity\"\npath = \"src/a.py\"\nname = \"nested\"\nvalue = 3\n"
        ),
        "{text}"
    );
}

#[test]
fn a_known_finding_is_left_out_while_its_value_does_not_grow() {
    let dir = project("known");
    write_baseline(&dir);

    let output = botica(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn a_known_finding_whose_value_grows_is_reported_with_the_value_of_the_baseline() {
    let dir = project("grows");
    write_baseline(&dir);
    write(&dir, "src/a.py", DEEPER);

    let output = botica(&dir, &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains(
            "`nested` has a cognitive complexity of 6 (the limit is 1): move nested conditions \
             and loops into functions of their own (the baseline holds 3)"
        ),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_function_that_the_baseline_does_not_list_is_reported() {
    let dir = project("new");
    write_baseline(&dir);
    write(&dir, "src/b.py", &NESTED.replace("nested", "other"));

    let output = botica(&dir, &[]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("`other`"), "{}", stdout(&output));
    assert!(!stdout(&output).contains("`nested`"), "{}", stdout(&output));
}

#[test]
fn the_baseline_matches_a_finding_seen_from_a_directory_inside_the_project() {
    let dir = project("below");
    write_baseline(&dir);

    let output = botica(&dir.join("src"), &["a.py"]);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn a_baseline_that_the_configuration_names_and_does_not_exist_fails_the_run() {
    let dir = project("missing");

    let output = botica(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("`botica lint --write-baseline` writes it"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn write_baseline_needs_the_configuration_the_root_and_no_paths() {
    let dir = project_in(
        "lint-baseline",
        "unnamed",
        Some("preview = true\nextend-select = [\"cognitive-complexity\"]\n"),
    );
    let output = botica(&dir, &["--write-baseline"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("needs `baseline = \"<path>\"`"),
        "{}",
        stderr(&output)
    );

    let dir = project("partial");
    let below = botica(&dir.join("src"), &["--write-baseline"]);
    assert_eq!(below.status.code(), Some(2));
    assert!(
        stderr(&below).contains("runs at the root of the project and takes no paths"),
        "{}",
        stderr(&below)
    );
    assert!(!dir.join("botica-baseline.toml").exists());
}
