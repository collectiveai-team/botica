#![cfg(all(feature = "fs", unix))]

use botica_astgrep::AstGrepSet;
use botica_core::walk::{discover, lint_streaming, read_all, read_and_lint, walk_and_lint};
use botica_core::{Engine, FileSizeGuard, NativeRule};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn engine() -> Engine {
    let native: Vec<Box<dyn NativeRule>> = vec![Box::new(FileSizeGuard::default())];
    Engine::new(native, AstGrepSet::default())
}

/// A tree with a file that is long, one that is short and one nobody may read. `None` when this
/// user reads the locked one anyway (root).
fn tree(test: &str) -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("unreadable")
        .join(test);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(root.join("pkg/big.txt"), "x = 1\n".repeat(701)).unwrap();
    fs::write(root.join("pkg/small.txt"), "x = 1\n").unwrap();
    let locked = root.join("pkg/locked.txt");
    fs::write(&locked, "secret\n").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&locked).is_ok() {
        eprintln!("skipped: the locked file is readable by this user");
        return None;
    }
    Some(root)
}

#[test]
fn walking_reports_a_file_it_cannot_read_apart_from_the_findings() {
    let Some(root) = tree("walk") else { return };

    let walked = walk_and_lint(&engine(), &root, &root);

    assert_eq!(walked.read, 2, "the locked file was not read");
    assert_eq!(walked.diagnostics.len(), 1, "the findings are untouched");
    assert_eq!(walked.unreadable.len(), 1);
    assert_eq!(walked.unreadable[0].path, "pkg/locked.txt");
    assert!(
        walked.unreadable[0].message.contains("Permission denied"),
        "{}",
        walked.unreadable[0].message
    );
}

#[test]
fn every_way_of_reading_named_files_reports_the_same_failure() {
    let Some(root) = tree("named") else { return };
    let engine = engine();
    let files = discover(&root, &root, &engine);

    let streamed = lint_streaming(&engine, &root, &files);
    let (_, two_passes) = read_and_lint(&engine, &root, &files);
    let (sources, unreadable) = read_all(&engine, &root, &files);

    assert_eq!(streamed.unreadable, two_passes.unreadable);
    assert_eq!(streamed.unreadable, unreadable);
    assert_eq!(unreadable.len(), 1);
    assert_eq!(unreadable[0].path, "pkg/locked.txt");
    assert_eq!(sources.len(), 2);
    assert_eq!(streamed.read, 2);
}

#[test]
fn a_file_that_is_gone_by_the_time_it_is_read_is_not_a_failure() {
    let Some(root) = tree("gone") else { return };
    let engine = engine();
    let files = discover(&root, &root, &engine);
    fs::remove_file(root.join("pkg/big.txt")).unwrap();

    let streamed = lint_streaming(&engine, &root, &files);
    let (_, two_passes) = read_and_lint(&engine, &root, &files);

    for walked in [&streamed, &two_passes] {
        assert_eq!(walked.read, 1, "only small.txt is left to read");
        assert_eq!(walked.unreadable.len(), 1, "only the locked file");
        assert_eq!(walked.unreadable[0].path, "pkg/locked.txt");
    }
}
