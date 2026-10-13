use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::{Engine, FileSizeGuard, NativeRule, NoUtils, Severity};

fn native_engine() -> Engine {
    let native: Vec<Box<dyn NativeRule>> = vec![
        Box::new(FileSizeGuard::default()),
        Box::new(NoUtils::default()),
    ];
    Engine::new(native, AstGrepSet::default())
}

fn lines(n: usize) -> String {
    "x = 1\n".repeat(n)
}

#[test]
fn file_size_guard_thresholds_match_the_python_hook() {
    let engine = native_engine();
    assert!(engine.lint_source("a.py", &lines(399)).is_empty());

    let warn = engine.lint_source("a.py", &lines(400));
    assert_eq!(warn.len(), 1);
    assert_eq!(warn[0].severity, Severity::Warning);

    let error = engine.lint_source("a.py", &lines(700));
    assert_eq!(error.len(), 1);
    assert_eq!(error[0].severity, Severity::Error);
}

#[test]
fn an_unterminated_last_line_counts_like_python() {
    let engine = native_engine();
    let text = format!("{}x = 1", "x = 1\n".repeat(399));
    assert_eq!(engine.lint_source("a.py", &text).len(), 1);
}

#[test]
fn no_utils_flags_catch_all_modules_outside_tests() {
    let engine = native_engine();
    assert_eq!(engine.lint_source("pkg/utils.py", "").len(), 1);
    assert!(engine.lint_source("helpers.ts", "").is_empty());
    assert!(engine.lint_source("common.js", "").is_empty());
    assert!(engine.lint_source("tests/utils.py", "").is_empty());
    assert!(engine.lint_source("pkg/utils_extra.py", "").is_empty());
    assert!(engine.lint_source("pkg/utils.md", "").is_empty());
}

#[test]
fn embedded_ast_grep_rule_keeps_its_composite_semantics() {
    let yaml = include_str!("../../../rules/logging/log-no-print/python.yml");
    let rules = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
    let engine = Engine::new(Vec::new(), rules);

    let source = "def f():\n    print('x')\n\nif __name__ == '__main__':\n    print('ok')\n";
    let found = engine.lint_source("pkg/lib.py", source);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (2, 5));

    assert!(engine.lint_source("pkg/cli.py", source).is_empty());
    assert!(engine.lint_source("tests/test_lib.py", source).is_empty());
}

#[test]
fn positions_are_reported_in_characters_after_non_ascii_text() {
    let yaml = include_str!("../../../rules/logging/log-no-print/python.yml");
    let rules = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
    let engine = Engine::new(Vec::new(), rules);

    let found = engine.lint_source("pkg/lib.py", "x = 'ñé'; print(x)\n");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (1, 11));
    assert_eq!(found[0].start, 12);
}
