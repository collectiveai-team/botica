use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lint-command")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the project");
    if let Some(text) = config {
        fs::write(dir.join("botica.toml"), text).expect("write the config");
    }
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parents");
    fs::write(path, text).expect("write the file");
}

fn lines(count: usize) -> String {
    "x = 1\n".repeat(count)
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .arg("lint")
        .args(args)
        .arg("--no-astgrep")
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

#[test]
fn file_size_guard_covers_every_language_and_skips_its_default_ignores() {
    let dir = project("size", None);
    write(&dir, "web/big.ts", &lines(701));
    write(&dir, "data/rows.txt", &lines(701));
    write(&dir, "Cargo.lock", &lines(701));
    write(&dir, "docs/guide.md", &lines(701));
    write(&dir, "tests/fixtures/a/b.json", &lines(701));
    let output = botica(&dir, &["."]);
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("web/big.ts:1:1: error [file-size-guard]"),
        "{text}"
    );
    assert!(
        text.contains("data/rows.txt:1:1: error [file-size-guard]"),
        "{text}"
    );
    for ignored in ["Cargo.lock", "guide.md", "b.json"] {
        assert!(
            !text.contains(ignored),
            "{ignored} is ignored by default:\n{text}"
        );
    }
}

#[test]
fn no_utils_only_applies_to_python() {
    let dir = project("utils", None);
    write(&dir, "pkg/utils.py", "");
    write(&dir, "web/utils.ts", "");
    let text = stdout(&botica(&dir, &["."]));
    assert!(
        text.contains("pkg/utils.py") && text.contains("[no-utils]"),
        "{text}"
    );
    assert!(!text.contains("web/utils.ts"), "{text}");
}

#[test]
fn the_config_changes_thresholds_ignores_and_the_selection() {
    let dir = project(
        "config",
        Some(
            "ignore = [\"no-utils\"]\n[rules.file-size-guard]\nwarn = 3\nmax = 6\nextend-ignores = [\"gen/**\"]\n",
        ),
    );
    write(&dir, "pkg/utils.py", "");
    write(&dir, "pkg/warn.py", &lines(4));
    write(&dir, "pkg/error.py", &lines(7));
    write(&dir, "gen/error.py", &lines(7));
    let output = botica(&dir, &["."]);
    let text = stdout(&output);
    assert!(
        text.contains("pkg/warn.py:1:1: warning [file-size-guard]"),
        "{text}"
    );
    assert!(
        text.contains("pkg/error.py:1:1: error [file-size-guard]"),
        "{text}"
    );
    assert!(text.contains("limit is 6"), "{text}");
    assert!(
        !text.contains("utils.py") && !text.contains("gen/"),
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn warnings_alone_do_not_fail_the_run() {
    let dir = project("warnings", Some("[rules.file-size-guard]\nwarn = 3\n"));
    write(&dir, "pkg/warn.py", &lines(4));
    let output = botica(&dir, &["."]);
    assert!(stdout(&output).contains("warning [file-size-guard]"));
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn a_path_that_does_not_exist_is_an_io_error_and_the_rest_is_still_checked() {
    let dir = project("missing", None);
    write(&dir, "big.py", &lines(701));
    let output = botica(&dir, &["typo/", "big.py", "typo.py"]);
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("typo/:1:1: error [io-error] No such file or directory (os error 2)"),
        "{text}"
    );
    assert!(text.contains("typo.py:1:1: error [io-error]"), "{text}");
    assert!(
        text.contains("big.py:1:1: error [file-size-guard]"),
        "{text}"
    );
}

#[test]
fn io_error_can_be_switched_off_in_the_config() {
    let dir = project("io-off", Some("ignore = [\"io-error\"]\n"));
    let output = botica(&dir, &["typo/"]);
    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(0));
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_is_an_io_error() {
    use std::os::unix::fs::PermissionsExt;
    let dir = project("unreadable", None);
    write(&dir, "locked.py", "x = 1\n");
    let path = dir.join("locked.py");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod");
    if fs::File::open(&path).is_ok() {
        eprintln!("skipped: the permission bits do not stop this user from reading");
        return;
    }
    let text = stdout(&botica(&dir, &["locked.py"]));
    assert!(
        text.contains("locked.py:1:1: error [io-error] Permission denied (os error 13)"),
        "{text}"
    );
}

#[test]
fn binary_files_are_not_reported() {
    let dir = project("binary", None);
    let mut blob = vec![0u8; 16];
    blob.extend("x\n".repeat(2_000).bytes());
    fs::write(dir.join("blob.bin"), blob).expect("write the blob");
    let output = botica(&dir, &["."]);
    assert_eq!(stdout(&output), "");
}

#[test]
fn paths_are_matched_from_the_project_root_even_when_run_from_a_subdirectory() {
    let dir = project(
        "subdir",
        Some("[rules.file-size-guard]\nextend-ignores = [\"sub/gen/**\"]\n"),
    );
    write(&dir, "sub/gen/big.py", &lines(701));
    write(&dir, "sub/big.py", &lines(701));
    let output = botica(&dir.join("sub"), &["."]);
    let text = stdout(&output);
    assert!(
        text.contains("big.py:1:1: error [file-size-guard]"),
        "{text}"
    );
    assert!(!text.contains("gen/"), "{text}");
    assert!(
        !text.contains("sub/"),
        "paths are printed from where botica runs:\n{text}"
    );
}

#[test]
fn an_explicit_config_and_an_invalid_one() {
    let dir = project("explicit", None);
    write(&dir, "pkg/utils.py", "");
    fs::write(dir.join("other.toml"), "ignore = [\"no-utils\"]\n").expect("write the config");
    let text = stdout(&botica(&dir, &[".", "--config", "other.toml"]));
    assert!(!text.contains("utils.py"), "{text}");

    fs::write(dir.join("bad.toml"), "select = [\"no-such-rule\"]\n").expect("write the config");
    let output = botica(&dir, &[".", "--config", "bad.toml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("select[0]"));
}

#[test]
fn hidden_files_and_gitignored_files_are_still_skipped() {
    let dir = project("walk", None);
    write(&dir, ".hidden/big.py", &lines(701));
    write(&dir, "ignored/big.py", &lines(701));
    write(&dir, ".gitignore", "ignored/\n");
    assert_eq!(stdout(&botica(&dir, &["."])), "");
}
