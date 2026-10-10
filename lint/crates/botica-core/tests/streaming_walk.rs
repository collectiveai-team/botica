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

/// One directory per test: the tests run in parallel and each rebuilds its tree.
fn tree(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("streaming_walk")
        .join(test);
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
    fs::write(root.join("pkg/blob.bin"), b"\0not text\n").unwrap();
    fs::write(root.join(".hidden/secret.py"), "x = 1\n".repeat(800)).unwrap();
    root
}

fn rendered(found: &[botica_core::Diagnostic]) -> Vec<String> {
    found.iter().map(ToString::to_string).collect()
}

#[test]
fn walking_and_linting_in_one_pass_reports_what_the_two_passes_report() {
    let root = tree("one_pass");
    let engine = engine();

    let files = discover(&root, &root, &engine);
    let (sources, two_passes) = read_and_lint(&engine, &root, &files);
    let one_pass = walk_and_lint(&engine, &root, &root);

    assert_eq!(
        rendered(&one_pass.diagnostics),
        rendered(&two_passes.diagnostics)
    );
    assert_eq!(one_pass.read, sources.len());
    assert_eq!(one_pass.read, two_passes.read);
}

#[test]
fn files_of_any_size_are_read_whole_and_binary_ones_are_judged_by_their_name() {
    let root = tree("any_size");
    let engine = engine();

    let walked = walk_and_lint(&engine, &root, &root);

    // big, small, empty, utils, the Markdown and the Latin-1 file (every native rule runs on
    // every path without a planner; a file that is not UTF-8 is still read, for the rules that
    // count bytes), and the binary file, which only the rules that go by its name see; not the
    // hidden directory.
    assert_eq!(walked.read, 7);
    assert!(walked.unreadable.is_empty());
    let messages = rendered(&walked.diagnostics).join("\n");
    assert!(messages.contains("is too large (40000 lines"), "{messages}");
    assert!(messages.contains("[no-utils]"), "{messages}");
    assert!(!messages.contains("secret.py"), "{messages}");
}

#[test]
fn explicit_files_are_linted_like_walked_ones() {
    let root = tree("explicit");
    let engine = engine();
    let files = vec![
        root.join("pkg/big.py"),
        root.join("pkg/latin1.py"),
        root.join("pkg/blob.bin"),
    ];

    let streamed = lint_streaming(&engine, &root, &files);

    assert_eq!(streamed.read, 3);
    let walked = walk_and_lint(&engine, &root, &root);
    let big = |d: &String| d.contains("big.py");
    assert_eq!(
        rendered(&streamed.diagnostics),
        rendered(&walked.diagnostics)
            .into_iter()
            .filter(big)
            .collect::<Vec<_>>()
    );
}
