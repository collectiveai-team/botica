//! The files of `tests/src` of complexipy 8.1.0 measure as complexipy measures them. `expected.json`
//! holds the value of each function that `complexipy tests/src --no-ignore --output-format json`
//! gives.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ast_grep_language::SupportLang;
use botica_complexity::measure;
use serde::Deserialize;

#[derive(Deserialize)]
struct Expected {
    path: String,
    function: String,
    complexity: u32,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/complexipy")
}

fn python_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            python_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "py") {
            out.push(path);
        }
    }
}

#[test]
fn each_function_of_the_fixtures_of_complexipy_has_its_value() {
    let root = fixtures().join("src");
    let expected: Vec<Expected> =
        serde_json::from_str(&fs::read_to_string(fixtures().join("expected.json")).unwrap())
            .unwrap();
    let mut wanted: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for entry in expected {
        wanted
            .entry(entry.path)
            .or_default()
            .push((entry.function, entry.complexity));
    }

    wanted.values_mut().for_each(|functions| functions.sort());

    let mut files = Vec::new();
    python_files(&root, &mut files);
    assert!(files.len() > 20, "the fixtures of complexipy are missing");

    let mut found: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let mut functions: Vec<(String, u32)> =
            measure(SupportLang::Python, &fs::read_to_string(&file).unwrap())
                .into_iter()
                .map(|function| (function.name, function.complexity))
                .collect();
        functions.sort();
        if !functions.is_empty() {
            found.insert(relative, functions);
        }
    }
    assert_eq!(found, wanted);
}
