//! An editor's buffer: the engine keeps the syntax tree of the pattern rules between edits, and
//! the findings in an edited buffer are those of its text linted afresh.

use botica_astgrep::{AstGrepRule, AstGrepSet};
use botica_core::{Engine, FileSizeGuard, NativeRule};

fn engine() -> Engine {
    let rules = AstGrepRule::from_yaml(include_str!(
        "../../../rules/logging/log-no-print/python.yml"
    ))
    .expect("the rule parses");
    let native: Vec<Box<dyn NativeRule>> = vec![Box::new(FileSizeGuard {
        warn_at: 3,
        error_at: 9,
    })];
    Engine::new(native, AstGrepSet::new(rules))
}

#[test]
fn a_buffer_of_a_language_of_the_pattern_rules_keeps_its_syntax_tree() {
    let engine = engine();

    assert!(engine.open("pkg/a.py", "x = 1\n").is_parsed());
    assert!(!engine.open("notes.md", "x\n").is_parsed());
}

#[test]
fn an_edited_buffer_has_the_findings_of_its_new_text() {
    let engine = engine();
    let mut buffer = engine.open("pkg/a.py", "x = 1\n");

    buffer.edit(0, 5, "print(1)\ny = 2\nz = 3");

    assert_eq!(buffer.text(), "print(1)\ny = 2\nz = 3\n");
    assert!(buffer.is_parsed());
    assert_eq!(
        engine.lint_buffer(&buffer),
        engine.lint_source("pkg/a.py", "print(1)\ny = 2\nz = 3\n")
    );
    assert_eq!(engine.lint_buffer(&buffer).len(), 2);
}

#[test]
fn a_buffer_given_a_whole_new_text_has_its_findings() {
    let engine = engine();
    let mut buffer = engine.open("pkg/a.py", "print(1)\n");

    buffer.replace("y = 2\n");

    assert_eq!(buffer.text(), "y = 2\n");
    assert!(engine.lint_buffer(&buffer).is_empty());
}

#[test]
fn an_edit_off_the_text_or_inside_a_character_lands_on_the_nearest_character_before() {
    let engine = engine();
    let mut buffer = engine.open("pkg/a.py", "ñ = 1\n");

    buffer.edit(1, 99, "x");

    assert_eq!(buffer.text(), "x");
}
