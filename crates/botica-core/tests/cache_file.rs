#![cfg(feature = "fs")]

use botica_core::cache::Cache;
use std::fs;
use std::path::{Path, PathBuf};

/// One empty directory per test: the tests run in parallel.
fn scratch(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cache_file")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    found.sort();
    found
}

#[test]
fn a_save_that_fails_leaves_no_temporary_file_behind() {
    let dir = scratch("failed");
    // The name of the cache is taken by a directory, so the rename cannot succeed.
    fs::create_dir(dir.join("lint-cache.json")).unwrap();

    assert!(Cache::load(&dir, "first").save().is_err());

    assert_eq!(names(&dir), ["lint-cache.json"]);
}

/// A second process that reads the cache while another one saves it must see the old file or
/// the new one, never half of each: a save replaces the file, it does not rewrite it in place.
#[test]
fn saving_replaces_the_file_instead_of_rewriting_it_in_place() {
    let dir = scratch("replace");
    Cache::load(&dir, "first").save().unwrap();
    let file = dir.join("lint-cache.json");
    // A reader that still holds the old file under another name.
    let held = dir.join("held.json");
    fs::hard_link(&file, &held).unwrap();
    let before = fs::read(&held).unwrap();

    Cache::load(&dir, "second").save().unwrap();

    assert_eq!(
        fs::read(&held).unwrap(),
        before,
        "the old file was rewritten"
    );
    assert_ne!(fs::read(&file).unwrap(), before, "the cache was not saved");
    assert_eq!(names(&dir), ["held.json", "lint-cache.json"]);
}
