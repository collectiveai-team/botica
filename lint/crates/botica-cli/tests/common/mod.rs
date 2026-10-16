#![allow(dead_code)]

pub mod fresh_rule;
pub mod lsp;
pub mod new_rule;
pub mod node;
pub mod project_catalog;

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

pub fn write_bytes(dir: &Path, relative: &str, bytes: &[u8]) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parents");
    fs::write(path, bytes).expect("write the file");
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

/// A subcommand of botica other than `lint`, as given.
pub fn command(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

/// A shell script `ext.sh` that speaks the extension protocol and answers every `lint/files` with
/// `replies_with` (the items of the `diagnostics` array).
pub fn fake_extension(dir: &Path, replies_with: &str) {
    write(
        dir,
        "ext.sh",
        &format!(
            r#"while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":1,"result":{{"rules":[{{"id":"ext-rule","languages":["python"]}}]}}}}' ;;
    *'"lint/files"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":2,"result":{{"diagnostics":[{replies_with}]}}}}' ;;
    *'"shutdown"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":3,"result":null}}'; exit 0 ;;
  esac
done
"#
        ),
    );
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 output")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}
pub mod extension;
