//! Python, as `crates/complexipy-core/src/cognitive_complexity.rs` and `utils.rs` of complexipy
//! 8.1.0 count it. complexipy walks the syntax tree of ruff; this module reads the one of
//! tree-sitter, and turns each expression into the shape ruff gives it before it counts.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

use crate::Function;

use expression::{count, expression, Expr};

mod expression;

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

pub(crate) fn measure(source: &str) -> Vec<Function> {
    let grep = SupportLang::Python.ast_grep(source);
    let root = grep.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return Vec::new();
    }
    let mut functions = Vec::new();
    for statement in statements(&root) {
        let definition = unwrap_decorated(statement);
        match &*definition.kind() {
            "function_definition" => functions.push(analyze(&definition, name_of(&definition))),
            "class_definition" => {
                let class = name_of(&definition);
                for method in body(&definition).iter().filter_map(function_of) {
                    let name = format!("{class}::{}", name_of(&method));
                    functions.push(analyze(&method, name));
                }
            }
            _ => {}
        }
    }
    functions
}

fn analyze(function: &PyNode<'_>, name: String) -> Function {
    let mut complexity = statement(function, 0);
    if calls_itself(function) {
        complexity += 1;
    }
    let range = function.range();
    Function {
        name,
        start: range.start,
        end: range.end,
        complexity,
    }
}

fn name_of(definition: &PyNode<'_>) -> String {
    definition
        .field("name")
        .map(|name| name.text().into_owned())
        .unwrap_or_default()
}

fn is_statement(node: &PyNode<'_>) -> bool {
    node.is_named() && node.kind() != "comment"
}

fn statements<'r>(block: &PyNode<'r>) -> Vec<PyNode<'r>> {
    block.children().filter(is_statement).collect()
}

fn body<'r>(definition: &PyNode<'r>) -> Vec<PyNode<'r>> {
    definition
        .field("body")
        .map(|block| statements(&block))
        .unwrap_or_default()
}

fn unwrap_decorated(node: PyNode<'_>) -> PyNode<'_> {
    if node.kind() == "decorated_definition" {
        if let Some(definition) = node.field("definition") {
            return definition;
        }
    }
    node
}

fn function_of<'r>(node: &PyNode<'r>) -> Option<PyNode<'r>> {
    let definition = unwrap_decorated(node.clone());
    (definition.kind() == "function_definition").then_some(definition)
}

/// A function whose body is a function and a `return`, the shape of a decorator, counts as the
/// function it holds.
fn decorated_inner<'r>(function: &PyNode<'r>) -> Option<PyNode<'r>> {
    let body = body(function);
    match body.as_slice() {
        [inner, last] if last.kind() == "return_statement" => {
            function_of(inner).map(|_| inner.clone())
        }
        _ => None,
    }
}

fn suite(block: Option<PyNode<'_>>, nesting: u32) -> u32 {
    block
        .map(|block| {
            statements(&block)
                .iter()
                .map(|node| statement(node, nesting))
                .sum()
        })
        .unwrap_or(0)
}

fn statement(node: &PyNode<'_>, nesting: u32) -> u32 {
    let node = unwrap_decorated(node.clone());
    match &*node.kind() {
        "function_definition" => {
            if let Some(inner) = decorated_inner(&node) {
                return statement(&inner, nesting);
            }
            body(&node)
                .iter()
                .map(|child| {
                    let deeper = function_of(child).is_some();
                    statement(child, nesting + u32::from(deeper))
                })
                .sum()
        }
        "class_definition" => body(&node)
            .iter()
            .filter(|child| function_of(child).is_some())
            .map(|child| statement(child, nesting))
            .sum(),
        "expression_statement" => expression_statement(&node, nesting),
        "for_statement" => loop_statement(&node, node.field("right"), nesting),
        "while_statement" => loop_statement(&node, node.field("condition"), nesting),
        "if_statement" => if_statement(&node, nesting),
        "try_statement" => try_statement(&node, nesting),
        "match_statement" => {
            let cases: u32 = node
                .field("body")
                .map(|block| {
                    block
                        .children()
                        .filter(|case| case.kind() == "case_clause")
                        .map(|case| suite(case.field("consequence"), nesting + 1))
                        .sum()
                })
                .unwrap_or(0);
            1 + nesting + cases
        }
        "return_statement" | "raise_statement" | "assert_statement" => node
            .children()
            .filter(is_statement)
            .map(|child| count(&expression(&child), nesting))
            .sum(),
        "with_statement" => {
            let items: u32 = node
                .children()
                .filter(|child| child.kind() == "with_clause")
                .flat_map(|clause| clause.children().collect::<Vec<_>>())
                .filter(|item| item.kind() == "with_item")
                .filter_map(|item| item.field("value"))
                .map(|value| {
                    let context = if value.kind() == "as_pattern" {
                        value.children().find(is_statement)
                    } else {
                        None
                    };
                    count(&expression(context.as_ref().unwrap_or(&value)), nesting)
                })
                .sum();
            items + suite(node.field("body"), nesting)
        }
        _ => 0,
    }
}

fn expression_statement(node: &PyNode<'_>, nesting: u32) -> u32 {
    let children: Vec<PyNode<'_>> = node.children().filter(is_statement).collect();
    let value = match children.as_slice() {
        [single] if single.kind() == "assignment" => {
            let mut assignment = single.clone();
            loop {
                match assignment.field("right") {
                    Some(right) if right.kind() == "assignment" => assignment = right,
                    Some(right) => break expression(&right),
                    None => break Expr::Other,
                }
            }
        }
        [single] if single.kind() == "augmented_assignment" => single
            .field("right")
            .map_or(Expr::Other, |right| expression(&right)),
        [single] => expression(single),
        many => Expr::Elements(many.iter().map(expression).collect()),
    };
    count(&value, nesting)
}

fn loop_statement(node: &PyNode<'_>, control: Option<PyNode<'_>>, nesting: u32) -> u32 {
    let boolean = control.map_or(0, |control| count(&expression(&control), nesting));
    let orelse = node
        .field("alternative")
        .map_or(0, |clause| suite(clause.field("body"), nesting));
    1 + nesting + boolean + suite(node.field("body"), nesting + 1) + orelse
}

fn if_statement(node: &PyNode<'_>, nesting: u32) -> u32 {
    let test = node
        .field("condition")
        .map_or(0, |test| count(&expression(&test), nesting));
    let mut complexity = 1 + nesting + test + suite(node.field("consequence"), nesting + 1);
    for clause in node.field_children("alternative") {
        complexity += match &*clause.kind() {
            "elif_clause" => {
                let test = clause
                    .field("condition")
                    .map_or(0, |test| count(&expression(&test), nesting));
                1 + test + suite(clause.field("consequence"), nesting + 1)
            }
            _ => 1 + suite(clause.field("body"), nesting + 1),
        };
    }
    complexity
}

fn try_statement(node: &PyNode<'_>, nesting: u32) -> u32 {
    let mut complexity = suite(node.field("body"), nesting);
    for clause in node.children() {
        complexity += match &*clause.kind() {
            "except_clause" | "except_group_clause" => {
                let block = clause.children().find(|child| child.kind() == "block");
                1 + nesting + suite(block, nesting + 1)
            }
            "else_clause" => suite(clause.field("body"), nesting),
            "finally_clause" => suite(
                clause.children().find(|child| child.kind() == "block"),
                nesting,
            ),
            _ => 0,
        };
    }
    complexity
}

/// Whether the body of `function` calls it by its name, outside the functions, classes and
/// lambdas it holds.
fn calls_itself(function: &PyNode<'_>) -> bool {
    let name = name_of(function);
    let Some(block) = function.field("body") else {
        return false;
    };
    let mut pending = vec![block];
    while let Some(node) = pending.pop() {
        match &*node.kind() {
            "function_definition" | "class_definition" | "decorated_definition" | "lambda" => {
                continue
            }
            "call" => {
                let named = node
                    .field("function")
                    .is_some_and(|callee| callee.kind() == "identifier" && callee.text() == name);
                if named {
                    return true;
                }
            }
            _ => {}
        }
        pending.extend(node.children());
    }
    false
}
