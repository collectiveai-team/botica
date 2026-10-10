mod common;

use common::{botica, command, fake_extension, project_in, stdout, write, write_bytes};
use std::path::PathBuf;

fn project(name: &str) -> PathBuf {
    let dir = project_in("non-utf8", name, None);
    std::fs::canonicalize(dir).expect("the project exists")
}

/// `é` in Latin-1 is a single byte, 0xE9, which is not valid UTF-8 on its own.
fn latin1_lines(count: usize) -> Vec<u8> {
    b"caf\xE9\n".repeat(count)
}

#[test]
fn a_latin1_text_file_over_the_limit_is_reported() {
    let dir = project("latin1-txt");
    write_bytes(&dir, "notes.txt", &latin1_lines(701));

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    let text = stdout(&output);
    assert!(text.contains("notes.txt"), "{text}");
    assert!(text.contains("[file-size-guard]"), "{text}");
    assert!(text.contains("701 lines"), "{text}");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn the_cached_path_reports_a_latin1_file_on_a_miss_and_on_a_hit() {
    let dir = project("cache-latin1");
    write_bytes(&dir, "notes.txt", &latin1_lines(701));
    let cache = dir.join("cache");
    let cache = cache.to_str().expect("UTF-8 path");

    for run in ["miss", "hit"] {
        let output = command(&dir, &["lint", ".", "--no-astgrep", "--cache-dir", cache]);
        let text = stdout(&output);
        assert!(text.contains("notes.txt"), "{run}:\n{text}");
        assert!(text.contains("[file-size-guard]"), "{run}:\n{text}");
        assert_eq!(output.status.code(), Some(1), "{run}");
    }
}

#[test]
fn a_run_with_an_extension_still_reports_a_latin1_file() {
    let dir = project("ext-latin1");
    write_bytes(&dir, "big.py", &latin1_lines(701));
    write(&dir, "ok.py", "x = 1\n");
    fake_extension(
        &dir,
        r#"{"path":"ok.py","rule":"ext-rule","severity":"warning","start":0,"end":1,"message":"from ext"}"#,
    );

    let output = botica(&dir, &[".", "--ext", "sh ext.sh"]);

    let text = stdout(&output);
    assert!(text.contains("big.py"), "{text}");
    assert!(text.contains("[file-size-guard]"), "{text}");
    assert!(text.contains("[ext-rule]"), "{text}");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn a_binary_file_over_the_limit_is_left_alone() {
    let dir = project("binary");
    write_bytes(&dir, "blob.bin", &b"\0abc\n".repeat(701));

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    assert_eq!(stdout(&output), "");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn a_python_file_that_is_not_utf8_gets_no_findings_from_ast_grep() {
    let dir = project("python-latin1");
    write(&dir, "pkg/ok.py", "print('caf\u{e9}')\n");
    write_bytes(&dir, "pkg/mod.py", b"print('caf\xE9')\n");

    let text = stdout(&command(&dir, &["lint", "."]));

    assert!(
        text.contains("pkg/ok.py"),
        "ast-grep is not running:\n{text}"
    );
    assert!(!text.contains("pkg/mod.py"), "{text}");
}

#[test]
fn suppression_comments_are_not_read_in_a_file_that_is_not_utf8() {
    let dir = project("comments-latin1");
    write(&dir, "ok.py", "x = 1  # botica: ignore[no-such-rule]\n");
    write_bytes(
        &dir,
        "mod.py",
        b"caf\xE9 = 1  # botica: ignore[no-such-rule]\n",
    );

    let text = stdout(&command(&dir, &["lint", ".", "--no-astgrep"]));

    assert!(
        text.contains("ok.py"),
        "the comments are not being read:\n{text}"
    );
    assert!(!text.contains("mod.py"), "{text}");
}

#[test]
fn a_catch_all_module_is_reported_whatever_its_encoding() {
    let dir = project("utils-latin1");
    write_bytes(&dir, "pkg/utils.py", b"caf\xE9 = 1\n");

    let text = stdout(&command(&dir, &["lint", ".", "--no-astgrep"]));

    assert!(text.contains("pkg/utils.py"), "{text}");
    assert!(text.contains("[no-utils]"), "{text}");
}
