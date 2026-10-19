#![cfg(feature = "fs")]

//! `walk::files`: every file of a project that a walk reaches, with no rule to keep or drop it.

use botica_core::walk::files;
use botica_core::{extensions_of, language_of, Exclusion};
use std::fs;
use std::path::{Path, PathBuf};

fn tree(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("project_files")
        .join(test);
    let _ = fs::remove_dir_all(&root);
    for dir in [".git", "src/pkg", "build", ".venv", ".github", "legacy"] {
        fs::create_dir_all(root.join(dir)).unwrap();
    }
    for file in [
        "src/a.py",
        "src/pkg/b.py",
        "src/pkg/notes.md",
        "build/out.py",
        ".venv/dep.py",
        ".github/ci.yml",
        "legacy/old.py",
    ] {
        fs::write(root.join(file), "x\n").unwrap();
    }
    fs::write(root.join(".gitignore"), "build/\n").unwrap();
    root
}

#[test]
fn every_file_the_walk_reaches_from_the_root_in_order() {
    let root = tree("all");

    let found = files(&root, None);

    assert_eq!(
        found,
        [
            ".github/ci.yml",
            ".gitignore",
            "legacy/old.py",
            "src/a.py",
            "src/pkg/b.py",
            "src/pkg/notes.md",
        ]
    );
}

#[test]
fn what_the_exclusion_names_is_left_out() {
    let root = tree("excluded");
    let exclusion = Exclusion {
        entry: Box::new(|path, is_dir| is_dir && path == "legacy"),
        path: Box::new(|path| path.ends_with(".md")),
    };

    let found = files(&root, Some(&exclusion));

    assert_eq!(
        found,
        [".github/ci.yml", ".gitignore", "src/a.py", "src/pkg/b.py"]
    );
}

#[test]
fn the_extensions_of_a_language_are_those_that_name_it() {
    for language in ["python", "typescript", "javascript"] {
        let extensions = extensions_of(language);
        assert!(!extensions.is_empty(), "{language}");
        for extension in extensions {
            assert_eq!(language_of(&format!("a.{extension}")), Some(language));
        }
    }
    assert_eq!(extensions_of("python"), ["py", "pyi"]);
    assert!(extensions_of("cobol").is_empty());
}
