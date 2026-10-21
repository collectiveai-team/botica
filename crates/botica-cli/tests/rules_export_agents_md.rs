//! `botica rules export --format agents-md`: the block of `AGENTS.md` with the judgment rules.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const START: &str = "<!-- botica:rules -->";
const END: &str = "<!-- /botica:rules -->";

fn workdir(name: &str, agents: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-export-agents-md")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the working directory");
    fs::write(dir.join("AGENTS.md"), agents).expect("write AGENTS.md");
    dir
}

fn export(dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec![
        "rules",
        "export",
        "--format",
        "agents-md",
        "--output",
        "AGENTS.md",
    ];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(&args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn agents(dir: &Path) -> String {
    fs::read_to_string(dir.join("AGENTS.md")).expect("read AGENTS.md")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_block_between_the_markers_is_replaced_and_the_rest_of_the_file_stays() {
    let dir = workdir(
        "replace",
        &format!("# Agent notes\n\n{START}\n- an old line\n{END}\n\nmore notes\n"),
    );

    let output = export(&dir, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        agents(&dir),
        format!("# Agent notes\n\n{START}\n{END}\n\nmore notes\n")
    );
}

#[test]
fn a_file_without_the_markers_is_left_as_it_is_and_the_export_fails() {
    let dir = workdir("no-markers", "# Agent notes\n");

    let output = export(&dir, &[]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains(&format!(
            "`AGENTS.md` needs a line `{START}` and, after it, a line `{END}`"
        )),
        "{}",
        stderr(&output)
    );
    assert_eq!(agents(&dir), "# Agent notes\n");
}

#[test]
fn check_passes_when_the_block_is_up_to_date_and_writes_nothing() {
    let text = format!("notes\n{START}\n{END}\n");
    let dir = workdir("check-ok", &text);

    let output = export(&dir, &["--check"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(agents(&dir), text);
}

#[test]
fn check_fails_when_the_block_differs_and_writes_nothing() {
    let text = format!("notes\n{START}\n- a line by hand\n{END}\n");
    let dir = workdir("check-stale", &text);

    let output = export(&dir, &["--check"]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains(
            "the block of `AGENTS.md` differs from the catalog and the configuration: run \
             `botica rules export --format agents-md --output AGENTS.md`"
        ),
        "{}",
        stderr(&output)
    );
    assert_eq!(agents(&dir), text);
}

#[test]
fn check_is_refused_with_a_format_that_has_no_check() {
    let dir = workdir("check-markdown", "");

    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args([
            "rules", "export", "--format", "markdown", "--output", "site", "--check",
        ])
        .current_dir(&dir)
        .output()
        .expect("run botica");

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("--check applies to --format agents-md"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.join("site").exists());
}
