use botica_astgrep::AstGrepSet;
use botica_core::{Engine, NoCoverageExclusion};
use std::collections::HashMap;

fn engine(bases: &[(&str, &str)]) -> Engine {
    let mut engine = Engine::new(vec![Box::new(NoCoverageExclusion)], AstGrepSet::default());
    engine.set_bases(
        bases
            .iter()
            .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
            .collect::<HashMap<_, _>>(),
    );
    engine
}

fn messages(engine: &Engine, path: &str, text: &str) -> Vec<String> {
    engine
        .lint_source(path, text)
        .into_iter()
        .map(|finding| format!("{}: {}", finding.line, finding.message))
        .collect()
}

#[test]
fn a_fail_under_lower_than_the_one_before_the_change_is_reported_on_its_line() {
    let before = "[tool.coverage.report]\nfail_under = 90\n";
    let engine = engine(&[
        ("pyproject.toml", before),
        ("svc/setup.cfg", "[coverage:report]\nfail_under = 85.5\n"),
    ]);

    assert_eq!(
        messages(&engine, "pyproject.toml", "[tool.coverage.report]\nfail_under = 80\n"),
        ["2: `fail_under` goes down from 90 to 80: keep 90, and cover the new code with tests or delete what nothing calls."]
    );
    assert_eq!(
        messages(
            &engine,
            "svc/setup.cfg",
            "[coverage:report]\nfail_under = 85\n"
        )
        .len(),
        1
    );
    assert!(messages(
        &engine,
        "pyproject.toml",
        "[tool.coverage.report]\nfail_under = 95\n"
    )
    .is_empty());
}

#[test]
fn without_a_base_a_fail_under_is_not_reported() {
    let engine = engine(&[]);

    assert!(messages(
        &engine,
        "pyproject.toml",
        "[tool.coverage.report]\nfail_under = 10\n"
    )
    .is_empty());
}

#[test]
fn files_coverage_does_not_read_and_sections_of_other_tools_are_left_alone() {
    let engine = engine(&[]);

    assert!(messages(&engine, "tox.ini", "[coverage:run]\nomit = a.py\n").is_empty());
    assert!(messages(&engine, "setup.cfg", "[run]\nomit = a.py\n").is_empty());
    assert!(messages(
        &engine,
        "pyproject.toml",
        "[tool.other]\nomit = [\"a.py\"]\n"
    )
    .is_empty());
    assert!(messages(&engine, "pyproject.toml", "not = [toml\n").is_empty());
    assert_eq!(
        messages(&engine, "x/.coveragerc", "[run]\nOMIT = a.py\n").len(),
        1
    );
}
