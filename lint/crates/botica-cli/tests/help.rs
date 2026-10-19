//! `botica --help`: each command says what it does.

mod common;

use common::{command, stdout};

#[test]
fn each_command_of_the_help_says_what_it_does() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));

    let text = stdout(&command(dir, &["--help"]));

    let lint = text
        .lines()
        .find(|line| line.trim_start().starts_with("lint "))
        .unwrap_or_else(|| panic!("no `lint` command: {text}"));
    assert!(
        lint.contains("Check the files of the project against the active rules"),
        "{lint}"
    );
}
