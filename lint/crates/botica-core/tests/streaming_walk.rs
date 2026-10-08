#![cfg(feature = "fs")]

use botica_astgrep::AstGrepSet;
use botica_core::walk::{discover, lint_streaming, read_and_lint, walk_and_lint};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils};
use std::fs;
use std::path::{Path, PathBuf};

fn engine() -> Engine {
    let native: Vec<Box<dyn NativeRule>> =
        vec![Box::new(FileSizeGuard::default()), Box::new(NoUtils)];
    Engine::new(native, AstGrepSet::default())
}

fn tree() -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("streaming_walk");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::create_dir_all(root.join(".hidden")).unwrap();
    // Several times the first read buffer, so the buffer has to grow.
    fs::write(root.join("pkg/big.py"), "x = 1\n".repeat(40_000)).unwrap();
    fs::write(root.join("pkg/small.py"), "x = 1\n").unwrap();
    fs::write(root.join("pkg/empty.py"), "").unwrap();
    fs::write(root.join("pkg/utils.py"), "").unwrap();
    fs::write(root.join("pkg/notes.md"), "not code\n").unwrap();
    fs::write(root.join("pkg/latin1.py"), [b'x', b' ', 0xE9, b'\n']).unwrap();
    fs::write(root.join(".hidden/secret.py"), "x = 1\n".repeat(800)).unwrap();
    root
}

fn rendered(found: &[botica_core::Diagnostic]) -> Vec<String> {
    found.iter().map(ToString::to_string).collect()
}

#[test]
fn walking_and_linting_in_one_pass_reports_what_the_two_passes_report() {
    let root = tree();
    let engine = engine();
    let never = |_: &Path| false;

    let files = discover(&root, &never);
    let (sources, two_passes) = read_and_lint(&engine, &root, &files);
    let (read, one_pass) = walk_and_lint(&engine, &root, &root, &never);

    assert_eq!(rendered(&one_pass), rendered(&two_passes));
    assert_eq!(read, sources.len());
}

#[test]
fn files_of_any_size_are_read_whole_and_unreadable_ones_are_skipped() {
    let root = tree();
    let engine = engine();
    let never = |_: &Path| false;

    let (read, found) = walk_and_lint(&engine, &root, &root, &never);

    // big, small, empty and utils; not the Markdown, the hidden directory or the non-UTF-8 file.
    assert_eq!(read, 4);
    let messages = rendered(&found).join("\n");
    assert!(messages.contains("is too large (40000 lines"), "{messages}");
    assert!(messages.contains("[no-utils]"), "{messages}");
    assert!(!messages.contains("secret.py"), "{messages}");
}

#[test]
fn explicit_files_are_linted_like_walked_ones() {
    let root = tree();
    let engine = engine();
    let never = |_: &Path| false;
    let files = vec![root.join("pkg/big.py"), root.join("pkg/latin1.py")];

    let (read, found) = lint_streaming(&engine, &root, &files);

    assert_eq!(read, 1);
    let (_, walked) = walk_and_lint(&engine, &root, &root, &never);
    let big = |d: &String| d.contains("big.py");
    assert_eq!(
        rendered(&found),
        rendered(&walked)
            .into_iter()
            .filter(big)
            .collect::<Vec<_>>()
    );
}
