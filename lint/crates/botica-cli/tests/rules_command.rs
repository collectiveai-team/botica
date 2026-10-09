use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-command")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the project");
    if let Some(text) = config {
        fs::write(dir.join("botica.toml"), text).expect("write the config");
    }
    dir
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

fn line_of<'a>(text: &'a str, slug: &str) -> &'a str {
    text.lines()
        .find(|line| line.split_whitespace().any(|word| word == slug))
        .unwrap_or_else(|| panic!("no line for {slug} in:\n{text}"))
}

#[test]
fn lists_the_active_rules_with_their_code_and_why() {
    let dir = project("default", None);
    let output = botica(&dir, &["rules"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);
    let line = line_of(&text, "file-size-guard");
    assert!(
        line.contains("BTSTRUC001") && line.contains("selected by `default`"),
        "{line}"
    );
    assert!(line_of(&text, "io-error").contains("BT001"));
}

#[test]
fn all_adds_the_inactive_rules_and_says_why() {
    let dir = project(
        "ignored",
        Some("ignore = [\"no-utils\"]\n[rules.file-size-guard]\nenabled = false\n"),
    );
    let default_view = stdout(&botica(&dir, &["rules"]));
    assert!(
        !default_view.contains("no-utils") && !default_view.contains("file-size-guard"),
        "{default_view}"
    );

    let text = stdout(&botica(&dir, &["rules", "--all"]));
    assert!(
        line_of(&text, "no-utils").contains("ignored by `no-utils`"),
        "{text}"
    );
    assert!(
        line_of(&text, "file-size-guard").contains("enabled = false"),
        "{text}"
    );
    assert!(line_of(&text, "io-error").contains("selected by"), "{text}");
}

#[test]
fn says_which_config_was_used() {
    let dir = project("with-config", Some("select = [\"BTSTRUC\"]\n"));
    let nested = dir.join("a/b");
    fs::create_dir_all(&nested).expect("create the nested directory");
    let text = stdout(&botica(&nested, &["rules"]));
    assert!(text.contains("botica.toml"), "{text}");
    assert!(
        !text.contains("io-error"),
        "only the structure family is selected:\n{text}"
    );

    let text = stdout(&botica(&project("no-config", None), &["rules"]));
    assert!(text.contains("defaults"), "{text}");
}

#[test]
fn an_explicit_config_wins() {
    let dir = project("explicit", Some("select = [\"all\"]\n"));
    fs::write(dir.join("other.toml"), "select = [\"BT001\"]\n").expect("write the other config");
    let text = stdout(&botica(&dir, &["rules", "--config", "other.toml"]));
    assert!(text.contains("other.toml"));
    assert!(
        text.contains("io-error") && !text.contains("no-utils"),
        "{text}"
    );
}

#[test]
fn an_invalid_config_exits_with_2_and_names_the_problem() {
    let dir = project("invalid", Some("select = [\"no-such-rule\"]\n"));
    let output = botica(&dir, &["rules"]);
    assert_eq!(output.status.code(), Some(2));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("no-such-rule") && message.contains("select[0]"),
        "{message}"
    );
}
