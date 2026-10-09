#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn project_in(group: &str, name: &str, config: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(group)
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the project");
    if let Some(text) = config {
        fs::write(dir.join("botica.toml"), text).expect("write the config");
    }
    dir
}

pub fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parents");
    fs::write(path, text).expect("write the file");
}

pub fn lines(count: usize) -> String {
    "x = 1\n".repeat(count)
}

pub fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .arg("lint")
        .args(args)
        .arg("--no-astgrep")
        .current_dir(dir)
        .output()
        .expect("run botica")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}
