//! The expressions of Python, in the shape that the syntax tree of ruff gives them, and what
//! complexipy counts in them.

use super::{is_statement, PyNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BoolKind {
    And,
    Or,
}

pub(super) struct Generator {
    iter: Expr,
    ifs: Vec<Expr>,
}

/// An expression in the shape of the syntax tree of ruff, keeping only what complexipy counts.
pub(super) enum Expr {
    BoolOp(BoolKind, Vec<Expr>),
    Unary(Box<Expr>),
    Compare(Vec<Expr>),
    Ternary {
        test: Box<Expr>,
        body: Box<Expr>,
        orelse: Box<Expr>,
    },
    Lambda(Box<Expr>),
    Comprehension {
        elements: Vec<Expr>,
        generators: Vec<Generator>,
    },
    /// The positional arguments of a call, the elements of a tuple, a list or a set, or the
    /// values of a dict.
    Elements(Vec<Expr>),
    Other,
}

pub(super) fn expression(node: &PyNode<'_>) -> Expr {
    Expr::from_node(node)
}

fn named(node: &PyNode<'_>) -> Vec<Expr> {
    node.children()
        .filter(is_statement)
        .map(|child| expression(&child))
        .collect()
}

impl Expr {
    fn from_node(node: &PyNode<'_>) -> Expr {
        match &*node.kind() {
            "parenthesized_expression" => node
                .children()
                .find(is_statement)
                .map_or(Expr::Other, |inner| expression(&inner)),
            "boolean_operator" => {
                let kind = bool_kind(node);
                let mut values = Vec::new();
                flatten(node, kind, &mut values);
                Expr::BoolOp(kind, values)
            }
            "not_operator" | "unary_operator" => {
                node.field("argument").map_or(Expr::Other, |argument| {
                    Expr::Unary(Box::new(expression(&argument)))
                })
            }
            "comparison_operator" => Expr::Compare(named(node)),
            "conditional_expression" => {
                let mut parts = named(node).into_iter();
                match (parts.next(), parts.next(), parts.next()) {
                    (Some(body), Some(test), Some(orelse)) => Expr::Ternary {
                        test: Box::new(test),
                        body: Box::new(body),
                        orelse: Box::new(orelse),
                    },
                    _ => Expr::Other,
                }
            }
            "lambda" => node.field("body").map_or(Expr::Other, |body| {
                Expr::Lambda(Box::new(expression(&body)))
            }),
            "list_comprehension"
            | "set_comprehension"
            | "generator_expression"
            | "dictionary_comprehension" => comprehension(node),
            "call" => match node.field("arguments") {
                Some(arguments) if arguments.kind() == "generator_expression" => {
                    Expr::Elements(vec![comprehension(&arguments)])
                }
                Some(arguments) => Expr::Elements(
                    arguments
                        .children()
                        .filter(is_statement)
                        .filter(|argument| {
                            !matches!(&*argument.kind(), "keyword_argument" | "dictionary_splat")
                        })
                        .map(|argument| expression(&argument))
                        .collect(),
                ),
                None => Expr::Other,
            },
            "tuple" | "list" | "set" | "expression_list" => Expr::Elements(named(node)),
            "dictionary" => Expr::Elements(
                node.children()
                    .filter_map(|item| match &*item.kind() {
                        "pair" => item.field("value").map(|value| expression(&value)),
                        "dictionary_splat" => item
                            .children()
                            .find(is_statement)
                            .map(|value| expression(&value)),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => Expr::Other,
        }
    }
}

fn bool_kind(node: &PyNode<'_>) -> BoolKind {
    match node.field("operator") {
        Some(operator) if operator.text() == "or" => BoolKind::Or,
        _ => BoolKind::And,
    }
}

/// The values of a chain of one boolean operator, as ruff gathers `a and b and c` into one node.
fn flatten(node: &PyNode<'_>, kind: BoolKind, values: &mut Vec<Expr>) {
    for side in ["left", "right"] {
        let Some(operand) = node.field(side) else {
            continue;
        };
        if operand.kind() == "boolean_operator" && bool_kind(&operand) == kind {
            flatten(&operand, kind, values);
        } else {
            values.push(expression(&operand));
        }
    }
}

fn comprehension(node: &PyNode<'_>) -> Expr {
    let mut elements = Vec::new();
    let mut generators: Vec<Generator> = Vec::new();
    for child in node.children().filter(is_statement) {
        match &*child.kind() {
            "for_in_clause" => {
                let mut iters: Vec<Expr> = child
                    .field_children("right")
                    .map(|right| expression(&right))
                    .collect();
                let iter = if iters.len() == 1 {
                    iters.remove(0)
                } else {
                    Expr::Elements(iters)
                };
                generators.push(Generator {
                    iter,
                    ifs: Vec::new(),
                });
            }
            "if_clause" => {
                if let (Some(generator), Some(test)) =
                    (generators.last_mut(), child.children().find(is_statement))
                {
                    generator.ifs.push(expression(&test));
                }
            }
            "pair" => {
                elements.extend(child.field("key").map(|key| expression(&key)));
                elements.extend(child.field("value").map(|value| expression(&value)));
            }
            _ => elements.push(expression(&child)),
        }
    }
    Expr::Comprehension {
        elements,
        generators,
    }
}

/// `count_bool_ops` of complexipy.
pub(super) fn count(expr: &Expr, nesting: u32) -> u32 {
    match expr {
        Expr::BoolOp(_, values) => {
            1 + values
                .iter()
                .map(|value| differing(value, expr))
                .sum::<u32>()
        }
        Expr::Unary(operand) => differing(operand, expr),
        Expr::Compare(operands) | Expr::Elements(operands) => {
            operands.iter().map(|operand| count(operand, nesting)).sum()
        }
        Expr::Ternary { test, body, orelse } => {
            1 + nesting
                + count(test, nesting)
                + count(body, nesting + 1)
                + count(orelse, nesting + 1)
        }
        Expr::Lambda(body) => count(body, nesting + 1),
        Expr::Comprehension {
            elements,
            generators,
        } => {
            let inner = nesting + 1;
            let loops: u32 = generators
                .iter()
                .map(|generator| {
                    1 + nesting
                        + count(&generator.iter, nesting)
                        + generator
                            .ifs
                            .iter()
                            .map(|test| 1 + count(test, inner))
                            .sum::<u32>()
                })
                .sum();
            loops
                + elements
                    .iter()
                    .map(|element| count(element, inner))
                    .sum::<u32>()
        }
        Expr::Other => 0,
    }
}

/// `count_different_childs_type` of complexipy: what a boolean operator or a `not` inside a
/// boolean operator adds.
fn differing(expr: &Expr, parent: &Expr) -> u32 {
    match (expr, parent) {
        (Expr::BoolOp(kind, _), Expr::BoolOp(parent_kind, parent_values)) => {
            u32::from(kind != parent_kind)
                + parent_values
                    .iter()
                    .map(|value| differing(value, expr))
                    .sum::<u32>()
        }
        (Expr::BoolOp(..), Expr::Unary(operand)) => 1 + differing(operand, expr),
        (Expr::Unary(_), Expr::BoolOp(_, parent_values)) => parent_values
            .iter()
            .map(|value| differing(value, expr))
            .sum(),
        (Expr::Unary(_), Expr::Unary(operand)) => differing(operand, expr),
        _ => 0,
    }
}
