#![cfg(feature = "fs")]

use botica_astgrep::AstGrepSet;
use botica_core::walk::{discover, walk_and_lint};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils, Plan};
use std::fs;
use std::path::{Path, PathBuf};

fn natives() -> Vec<Box<dyn NativeRule>> {
    vec![
        Box::new(FileSizeGuard {
            warn_at: 2,
            error_at: 4,
        }),
        Box::new(NoUtils),
    ]
}

fn rules_of(found: &[botica_core::Diagnostic]) -> Vec<&str> {
    found.iter().map(|d| d.rule.as_str()).collect()
}

#[test]
fn without_a_planner_every_native_rule_runs_on_every_path() {
    let engine = Engine::new(natives(), AstGrepSet::default());
    let found = engine.lint_source("utils.py", "a\nb\n");
    assert_eq!(rules_of(&found), ["file-size-guard", "no-utils"]);
    assert_eq!(
        engine.plan("notes.md"),
        Some(Plan {
            native: vec![0, 1],
            astgrep: false,
            content: true,
        })
    );
}

#[test]
fn a_planner_chooses_the_native_rules_of_each_path() {
    let engine = Engine::new(natives(), AstGrepSet::default()).with_planner(Box::new(|path| {
        if path.ends_with(".py") {
            vec![1]
        } else if path.starts_with("big/") {
            vec![0]
        } else {
            vec![]
        }
    }));

    assert_eq!(
        rules_of(&engine.lint_source("utils.py", "1\n2\n3\n")),
        ["no-utils"]
    );
    assert_eq!(
        rules_of(&engine.lint_source("big/a.txt", "1\n2\n3\n")),
        ["file-size-guard"]
    );
    assert!(engine.lint_source("x.md", "1\n2\n3\n").is_empty());
}

#[test]
fn a_path_no_rule_wants_has_no_plan() {
    let engine = Engine::new(natives(), AstGrepSet::default()).with_planner(Box::new(|_| vec![]));
    assert_eq!(engine.plan("a.py"), None);
}

fn tree(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("planner")
        .join(test);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("src/a.py"), "1\n2\n3\n").unwrap();
    fs::write(root.join("src/b.ts"), "1\n2\n3\n").unwrap();
    fs::write(root.join("docs/guide.md"), "1\n2\n3\n").unwrap();
    fs::write(root.join("README"), "1\n2\n3\n").unwrap();
    root
}

#[test]
fn the_walk_reads_only_the_files_the_planner_wants() {
    let root = tree("wants");
    let engine = Engine::new(natives(), AstGrepSet::default()).with_planner(Box::new(|path| {
        if path.starts_with("src/") {
            vec![0]
        } else {
            vec![]
        }
    }));

    let walked = walk_and_lint(&engine, &root, &root);
    assert_eq!(walked.read, 2, "only src/a.py and src/b.ts");
    let paths: Vec<&str> = walked.diagnostics.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(paths, ["src/a.py", "src/b.ts"]);

    let listed = discover(&root, &root, &engine);
    let names: Vec<String> = listed
        .iter()
        .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
        .collect();
    assert_eq!(names, ["src/a.py", "src/b.ts"]);
}

#[test]
fn every_kind_of_file_is_walked_when_a_rule_covers_every_language() {
    let root = tree("all");
    let engine = Engine::new(natives(), AstGrepSet::default()).with_planner(Box::new(|_| vec![0]));
    assert_eq!(walk_and_lint(&engine, &root, &root).read, 4);
}

#[test]
fn binary_files_are_not_read_whole() {
    let root = tree("binary");
    let mut binary = vec![b'x'; 100];
    binary.push(0);
    binary.extend(std::iter::repeat_n(b'y', 5_000_000));
    fs::write(root.join("src/blob.dat"), binary).unwrap();
    let engine = Engine::new(natives(), AstGrepSet::default()).with_planner(Box::new(|_| vec![0]));
    let walked = walk_and_lint(&engine, &root, &root);
    assert_eq!(
        walked.read, 5,
        "the blob is linted by its name, not read whole"
    );
    assert!(walked
        .diagnostics
        .iter()
        .all(|d| !d.path.ends_with("blob.dat")));
    assert!(
        walked.unreadable.is_empty(),
        "a binary file is not a failure"
    );
}
