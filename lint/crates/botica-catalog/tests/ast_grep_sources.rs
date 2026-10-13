//! The YAML of an ast-grep implementation: the catalog carries it and checks it.

mod common;

use common::{catalog, family, file, issues, rule, rule_with};

const AST_GREP_PYTHON: &str =
    "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n";

#[test]
fn an_ast_grep_implementation_carries_the_text_of_its_file() {
    let loaded = catalog(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
        vec![file(
            "logging/log-no-print/python.yml",
            "id: log-no-print\n",
        )],
    ])
    .expect("a rule whose file is there loads");

    let rule = loaded.resolve("log-no-print").expect("rule exists");
    assert_eq!(
        rule.implementations[0].source.as_deref(),
        Some("id: log-no-print\n")
    );
}

#[test]
fn a_native_implementation_has_no_source() {
    let loaded = catalog(vec![
        vec![family("structure", "STRUC")],
        rule("structure", "one", "BTSTRUC001"),
    ])
    .expect("loads");

    assert_eq!(loaded.rules()[0].implementations[0].source, None);
}

#[test]
fn an_implementation_whose_file_is_missing_is_rejected() {
    let message = issues(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
    ]);

    assert!(message.contains("python.yml"), "{message}");
    assert!(message.contains("log-no-print"), "{message}");
}

#[test]
fn an_ast_grep_implementation_must_name_its_file() {
    let no_file = "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n";
    let message = issues(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", no_file),
    ]);

    assert!(message.contains("file"), "{message}");
}

#[test]
fn the_id_of_an_ast_grep_yaml_must_be_the_slug_of_its_rule() {
    let other = issues(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
        vec![file("logging/log-no-print/python.yml", "id: no-print\n")],
    ]);
    assert!(other.contains("python.yml"), "{other}");
    assert!(other.contains("no-print"), "{other}");
    assert!(other.contains("log-no-print"), "{other}");

    let none = issues(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
        vec![file(
            "logging/log-no-print/python.yml",
            "language: python\n",
        )],
    ]);
    assert!(none.contains("python.yml"), "{none}");

    let quoted = catalog(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
        vec![file(
            "logging/log-no-print/python.yml",
            "id: \"log-no-print\"\nrule:\n  id: nested\n",
        )],
    ]);
    assert!(
        quoted.is_ok(),
        "a quoted top-level id is the id; a nested one is not"
    );
}

#[test]
fn a_yaml_file_no_implementation_names_is_rejected() {
    let message = issues(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", AST_GREP_PYTHON),
        vec![
            file("logging/log-no-print/python.yml", "id: a\n"),
            file("logging/log-no-print/stray.yml", "id: b\n"),
        ],
    ]);

    assert!(message.contains("stray.yml"), "{message}");
}
