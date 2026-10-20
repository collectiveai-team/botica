use std::collections::BTreeMap;

use botica_astgrep::AstGrepSet;
use botica_catalog::Ecosystem;
use botica_core::registry::{self, Context};
use botica_core::{CodeDir, Engine, NativeRule};

fn dir(path: &str, ecosystem: Ecosystem) -> CodeDir {
    CodeDir {
        path: path.to_owned(),
        ecosystem,
    }
}

fn engine(slug: &str, code_dirs: Vec<CodeDir>) -> Engine {
    let rule = registry::build(
        slug,
        &BTreeMap::new(),
        &Context {
            code_dirs,
            ..Context::default()
        },
    )
    .expect("the rule is native");
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

fn nothing() -> Vec<String> {
    Vec::new()
}

mod where_the_code_is {
    use super::*;

    #[test]
    fn a_directory_that_only_starts_like_a_code_directory_is_not_inside_it() {
        let engine = engine("no-notebooks", vec![dir("backend", Ecosystem::Python)]);

        assert_eq!(
            reported(
                &engine,
                &[
                    "backend2/a.ipynb",
                    "backend-old/a.ipynb",
                    "backen/a.ipynb",
                    "a.ipynb"
                ]
            ),
            nothing()
        );
        assert_eq!(reported(&engine, &["backend/a.ipynb"]), ["backend/a.ipynb"]);
    }

    #[test]
    fn a_code_directory_may_be_a_nested_path_and_there_may_be_several() {
        let engine = engine(
            "no-notebooks",
            vec![
                dir("services/api/app", Ecosystem::Python),
                dir("web/src", Ecosystem::Js),
            ],
        );

        assert_eq!(
            reported(
                &engine,
                &[
                    "services/api/app/a.ipynb",
                    "services/api/a.ipynb",
                    "services/worker/app/a.ipynb",
                    "web/src/deep/er/a.ipynb",
                    "web/a.ipynb",
                ]
            ),
            ["services/api/app/a.ipynb", "web/src/deep/er/a.ipynb"]
        );
    }

    #[test]
    fn the_working_directory_may_be_the_code_and_then_every_path_is_inside_it() {
        let engine = engine("no-notebooks", vec![dir("", Ecosystem::Python)]);

        assert_eq!(
            reported(&engine, &["a.ipynb", "deep/er/b.ipynb", "main.py"]),
            ["a.ipynb", "deep/er/b.ipynb"]
        );
    }

    #[test]
    fn with_no_code_directory_the_rules_do_nothing() {
        for slug in [
            "no-notebooks",
            "no-tests-in-package",
            "no-artifacts-in-package",
        ] {
            let engine = engine(slug, Vec::new());

            assert_eq!(
                reported(&engine, &["a.ipynb", "test_a.py", "backend/amarc/rows.csv"]),
                nothing(),
                "{slug}"
            );
        }
    }
}

mod tests_in_the_package {
    use super::*;

    #[test]
    fn only_the_code_of_a_python_ecosystem_is_looked_at() {
        let engine = engine(
            "no-tests-in-package",
            vec![
                dir("backend/amarc", Ecosystem::Python),
                dir("frontend/src", Ecosystem::Js),
                dir("crates/x/src", Ecosystem::Rust),
                dir("internal", Ecosystem::Go),
            ],
        );

        assert_eq!(
            reported(
                &engine,
                &[
                    "backend/amarc/test_a.py",
                    "frontend/src/a.test.ts",
                    "frontend/src/tests/a.ts",
                    "crates/x/src/tests/a.rs",
                    "internal/auth/auth_test.go",
                ]
            ),
            ["backend/amarc/test_a.py"]
        );
    }

    #[test]
    fn the_message_says_the_rule_and_the_fix() {
        let engine = engine(
            "no-tests-in-package",
            vec![dir("backend/amarc", Ecosystem::Python)],
        );

        let found = engine.lint_source("backend/amarc/tests/test_a.py", "");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule, "no-tests-in-package");
        assert!(found[0].file_level);
        assert_eq!(
            found[0].message,
            "backend/amarc/tests/test_a.py is a test inside the package — keep tests in a `tests/` directory beside the package, not in it."
        );
    }
}

mod artifacts_in_the_package {
    use super::*;

    #[test]
    fn a_file_directly_in_the_code_directory_is_inside_it() {
        let engine = engine(
            "no-artifacts-in-package",
            vec![dir("backend/amarc", Ecosystem::Python)],
        );

        assert_eq!(
            reported(&engine, &["backend/amarc/rows.csv", "backend/rows.csv"]),
            ["backend/amarc/rows.csv"]
        );
    }

    #[test]
    fn the_three_directory_names_count_only_in_python_and_only_for_what_is_not_code() {
        let python = engine(
            "no-artifacts-in-package",
            vec![dir("backend/amarc", Ecosystem::Python)],
        );
        let others = engine(
            "no-artifacts-in-package",
            vec![
                dir("frontend/src", Ecosystem::Js),
                dir("crates/x/src", Ecosystem::Rust),
                dir("internal", Ecosystem::Go),
                dir("src/main/java", Ecosystem::Jvm),
            ],
        );

        assert_eq!(
            reported(
                &python,
                &[
                    "backend/amarc/data/a.yaml",
                    "backend/amarc/a/resources/b.json",
                    "backend/amarc/reports/c.html",
                    "backend/amarc/data/d.py",
                    "backend/amarc/data/e.pyi",
                    "backend/amarc/resources/py.typed",
                    "backend/amarc/metadata/f.yaml",
                    "backend/amarc/database/f.yaml",
                ]
            ),
            [
                "backend/amarc/data/a.yaml",
                "backend/amarc/a/resources/b.json",
                "backend/amarc/reports/c.html"
            ]
        );
        assert_eq!(
            reported(
                &others,
                &[
                    "frontend/src/data/a.json",
                    "frontend/src/resources/b.yaml",
                    "frontend/src/reports/c.html",
                    "crates/x/src/data/a.json",
                    "internal/resources/b.yaml",
                    "src/main/java/reports/c.html",
                ]
            ),
            nothing()
        );
    }

    #[test]
    fn the_message_says_what_it_found_and_the_fix() {
        let engine = engine(
            "no-artifacts-in-package",
            vec![dir("backend/amarc", Ecosystem::Python)],
        );

        let data = engine.lint_source("backend/amarc/rows.csv", "");
        let under = engine.lint_source("backend/amarc/data/schema.yaml", "");

        assert_eq!(
            data[0].message,
            "backend/amarc/rows.csv is a data or model file inside the code of the project — keep data, models and reports outside it."
        );
        assert_eq!(
            under[0].message,
            "backend/amarc/data/schema.yaml is a file under a `data/` directory inside the code of the project — keep data, models and reports outside it."
        );
    }
}

#[test]
fn the_notebook_message_says_the_rule_the_standard_and_the_fix() {
    let engine = engine(
        "no-notebooks",
        vec![dir("backend/amarc", Ecosystem::Python)],
    );

    let found = engine.lint_source("backend/amarc/a.ipynb", "");

    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].message,
        "backend/amarc/a.ipynb is a notebook inside the code of the project — keep notebooks outside it."
    );
}

#[test]
fn the_registry_knows_which_rules_read_the_code_directories() {
    for slug in [
        "no-notebooks",
        "no-tests-in-package",
        "no-artifacts-in-package",
    ] {
        assert!(registry::uses_code_dirs(slug), "{slug}");
    }
    for slug in ["file-size-guard", "no-utils", "io-error"] {
        assert!(!registry::uses_code_dirs(slug), "{slug}");
    }
}
