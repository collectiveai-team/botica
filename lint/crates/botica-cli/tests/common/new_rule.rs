//! Running `botica rules new` against a copy of the catalog of this checkout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use botica_catalog::{read_files, Catalog};

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create a directory");
    for entry in fs::read_dir(from).expect("read the catalog").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy a file");
        }
    }
}

/// A working directory with a copy of the catalog of this checkout under `cat/`.
pub fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-new")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules"),
        &dir.join("cat"),
    );
    dir
}

pub fn new_rule(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["rules", "new"])
        .args(args)
        .args(["--catalog", "cat"])
        .current_dir(dir)
        .output()
        .expect("run botica")
}

pub fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

pub fn reload(dir: &Path) -> Catalog {
    let files = read_files(&dir.join("cat")).expect("read the catalog");
    Catalog::from_files(&files).unwrap_or_else(|error| panic!("the catalog loads:\n{error}"))
}

pub fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
