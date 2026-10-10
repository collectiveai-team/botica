use std::collections::BTreeMap;

use botica_astgrep::AstGrepSet;
use botica_core::registry::{self, Context};
use botica_core::{Engine, NativeRule};

fn engine(slug: &str, packages: &[&str]) -> Engine {
    let context = Context {
        packages: packages.iter().map(|&package| package.to_owned()).collect(),
    };
    let rule = registry::build(slug, &BTreeMap::new(), &context).expect("a native rule");
    let rules: Vec<Box<dyn NativeRule>> = vec![rule];
    Engine::new(rules, AstGrepSet::default())
}

fn reported(engine: &Engine, paths: &[&str]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| !engine.lint_source(path, "").is_empty())
        .map(|&path| path.to_owned())
        .collect()
}

#[test]
fn an_artifact_directory_is_found_at_any_depth_inside_a_package() {
    let engine = engine("repo-shape", &["backend"]);

    assert_eq!(
        reported(
            &engine,
            &[
                "backend/data/rows.csv",
                "backend/app/reports/q1/summary.txt",
                "backend/a/b/c/resources/logo.png",
            ]
        ),
        [
            "backend/data/rows.csv",
            "backend/app/reports/q1/summary.txt",
            "backend/a/b/c/resources/logo.png",
        ]
    );
}

#[test]
fn only_a_directory_with_that_exact_name_counts() {
    let engine = engine("repo-shape", &["backend"]);

    assert_eq!(
        reported(
            &engine,
            &[
                "backend/mydata/rows.csv",
                "backend/data_dir/rows.csv",
                "backend/app/dataset/rows.csv",
                "backend/Data/rows.csv",
            ]
        ),
        Vec::<String>::new()
    );
}

#[test]
fn a_file_named_like_an_artifact_directory_is_not_one() {
    let engine = engine("repo-shape", &["backend"]);

    assert_eq!(
        reported(
            &engine,
            &["backend/data", "backend/app/reports", "backend/data.csv"]
        ),
        Vec::<String>::new()
    );
}

#[test]
fn a_directory_that_only_starts_like_a_package_is_not_in_it() {
    let engine = engine("repo-shape", &["backend"]);

    assert_eq!(
        reported(
            &engine,
            &[
                "backend2/data/rows.csv",
                "backend-old/data/rows.csv",
                "data/rows.csv"
            ]
        ),
        Vec::<String>::new()
    );
}

#[test]
fn a_package_may_be_a_nested_path_and_there_may_be_several() {
    let engine = engine("repo-shape", &["services/api", "web"]);

    assert_eq!(
        reported(
            &engine,
            &[
                "services/api/data/rows.csv",
                "services/data/rows.csv",
                "services/worker/data/rows.csv",
                "web/reports/q1.txt",
            ]
        ),
        ["services/api/data/rows.csv", "web/reports/q1.txt"]
    );
}

#[test]
fn a_package_that_is_the_working_directory_holds_every_path() {
    let engine = engine("repo-shape", &[""]);

    assert_eq!(
        reported(
            &engine,
            &["data/rows.csv", "app/reports/q1.txt", "app/main.py"]
        ),
        ["data/rows.csv", "app/reports/q1.txt"]
    );
}

#[test]
fn with_no_package_the_rule_does_nothing() {
    let engine = engine("repo-shape", &[]);

    assert_eq!(
        reported(&engine, &["backend/data/rows.csv", "data/rows.csv"]),
        Vec::<String>::new()
    );
}

#[test]
fn the_message_says_the_rule_the_standard_and_the_fix() {
    let engine = engine("repo-shape", &["backend"]);

    let found = engine.lint_source("backend/data/rows.csv", "");

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, "repo-shape");
    assert!(found[0].file_level);
    assert_eq!(
        found[0].message,
        "CES-32 (repo-shape): backend/data/rows.csv is under a `data/` directory inside a package — keep data, reports and resources outside the package."
    );
}

#[test]
fn a_notebook_is_found_at_any_depth_inside_a_package_and_only_there() {
    let engine = engine("no-notebooks", &["backend"]);

    assert_eq!(
        reported(
            &engine,
            &[
                "backend/analysis.ipynb",
                "backend/a/b/c/explore.ipynb",
                "notebooks/explore.ipynb",
                "backend2/explore.ipynb",
                "backend/notes.ipynb.txt",
                "backend/ipynb",
            ]
        ),
        ["backend/analysis.ipynb", "backend/a/b/c/explore.ipynb"]
    );
}

#[test]
fn a_notebook_needs_no_package_name_to_be_found_when_the_package_is_the_working_directory() {
    let engine = engine("no-notebooks", &[""]);

    assert_eq!(
        reported(&engine, &["explore.ipynb", "main.py"]),
        ["explore.ipynb"]
    );
}
