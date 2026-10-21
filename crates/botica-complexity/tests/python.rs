//! Python: the cases of `TestPaperConformance` in `tests/main.py` of complexipy 8.1.0, and what a
//! function reports besides its value. A case that complexipy writes at the top of a module is
//! written here inside `def f():`, since only functions are measured.

use ast_grep_language::SupportLang;
use botica_complexity::{measure, Function};

fn functions(source: &str) -> Vec<Function> {
    measure(SupportLang::Python, source)
}

fn value(source: &str) -> u32 {
    let functions = functions(source);
    assert_eq!(functions.len(), 1, "{source}");
    functions[0].complexity
}

#[test]
fn a_match_and_its_cases_add_one() {
    let source = "def f(x):\n    match x:\n        case 1:\n            return 'one'\n        case _:\n            return 'other'\n";
    assert_eq!(value(source), 1);
}

#[test]
fn a_match_in_a_loop_adds_its_nesting() {
    let source =
        "def f(xs, x):\n    for i in xs:\n        match x:\n            case 1:\n                pass\n";
    assert_eq!(value(source), 3);
}

#[test]
fn the_body_of_a_try_is_not_nested() {
    let source = "def f(x):\n    try:\n        if x:\n            pass\n    except Exception:\n        pass\n";
    assert_eq!(value(source), 2);
}

#[test]
fn a_finally_is_not_nested() {
    let source =
        "def f(x):\n    try:\n        pass\n    finally:\n        if x:\n            pass\n";
    assert_eq!(value(source), 1);
}

#[test]
fn an_except_in_a_loop_adds_its_nesting() {
    let source = "def f(xs):\n    for i in xs:\n        try:\n            pass\n        except Exception:\n            pass\n";
    assert_eq!(value(source), 3);
}

#[test]
fn a_with_does_not_nest() {
    assert_eq!(
        value("def f(x, y):\n    with open(x):\n        if y:\n            pass\n"),
        1
    );
}

#[test]
fn a_direct_recursive_call_adds_one() {
    assert_eq!(value("def fact(n):\n    return fact(n - 1)\n"), 1);
    assert_eq!(
        value("def fact(n):\n    if n:\n        return fact(n - 1)\n    return 1\n"),
        2
    );
}

#[test]
fn a_nested_function_that_calls_the_outer_one_is_not_recursion() {
    assert_eq!(
        value("def foo():\n    def bar():\n        foo()\n    return bar\n"),
        0
    );
}

#[test]
fn a_lambda_raises_the_nesting() {
    assert_eq!(value("def f():\n    g = lambda x: (1 if x else 2)\n"), 2);
    assert_eq!(value("def f():\n    g = lambda x: x and x\n"), 1);
}

#[test]
fn a_comprehension_counts_its_loops_and_filters() {
    assert_eq!(
        value("def f(xs):\n    return [x for x in xs if x > 0]\n"),
        2
    );
    assert_eq!(
        value("def f(xs):\n    return [y for x in xs for y in x if y]\n"),
        3
    );
    assert_eq!(
        value("def f(xs):\n    return any(x and x for x in xs)\n"),
        2
    );
}

#[test]
fn a_nested_ternary_adds_its_nesting() {
    assert_eq!(value("def f():\n    x = 1 if a else (2 if b else 3)\n"), 3);
}

#[test]
fn a_boolean_sequence_in_an_expression_adds_one() {
    assert_eq!(value("def f(a, b):\n    foo(a and b)\n"), 1);
}

#[test]
fn the_else_of_a_loop_is_not_nested() {
    let source = "def f(xs, x):\n    for i in xs:\n        pass\n    else:\n        if x:\n            pass\n";
    assert_eq!(value(source), 2);
}

#[test]
fn a_change_of_boolean_operator_adds_one() {
    assert_eq!(value("def f(a, b, c):\n    return a and b and c\n"), 1);
    assert_eq!(value("def f(a, b, c):\n    return a and b or c\n"), 2);
}

#[test]
fn a_method_is_named_after_its_class_and_starts_at_its_def() {
    let source = "class Order:\n    @property\n    def total(self):\n        return 0\n";
    let found = functions(source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "Order::total");
    assert_eq!(&source[found[0].start..found[0].start + 9], "def total");
    assert_eq!(found[0].end, source.len() - 1);
}

#[test]
fn a_nested_function_adds_to_the_one_that_holds_it() {
    let source = "def outer(x):\n    def inner(y):\n        if y:\n            pass\n    print(x)\n    return inner\n";
    let found = functions(source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "outer");
    assert_eq!(found[0].complexity, 2);
}

#[test]
fn a_source_that_does_not_parse_has_no_functions() {
    assert!(functions("def f(:\n    pass\n").is_empty());
}
