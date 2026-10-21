//! Rust.

use ast_grep_language::{LanguageExt, SupportLang};

use crate::structured::{self, text_of, Grammar, Role, SyntaxNode, Unit};
use crate::Function;

struct Rust;

impl Grammar for Rust {
    fn role<'r>(&self, node: &SyntaxNode<'r>) -> Role<'r> {
        match &*node.kind() {
            "if_expression" | "if_let_expression" => Role::If {
                header: node
                    .field("condition")
                    .into_iter()
                    .chain(node.field("pattern"))
                    .chain(node.field("value"))
                    .collect(),
                consequence: node.field("consequence"),
                alternative: node.field("alternative"),
            },
            "loop_expression"
            | "while_expression"
            | "while_let_expression"
            | "for_expression"
            | "match_expression" => Role::Structural {
                nested: node.field("body").into_iter().collect(),
            },
            "binary_expression" if self.logical_operator(node).is_some() => Role::Logical,
            "let_chain" => Role::Flat,
            "break_expression" | "continue_expression"
                if node.children().any(|child| child.kind() == "label") =>
            {
                Role::Flat
            }
            "closure_expression" | "function_item" => Role::Function,
            _ => Role::Plain,
        }
    }

    fn logical_operator(&self, node: &SyntaxNode<'_>) -> Option<String> {
        if node.kind() != "binary_expression" {
            return None;
        }
        let operator = text_of(node.field("operator"));
        matches!(operator.as_str(), "&&" | "||").then_some(operator)
    }

    fn is_if(&self, node: &SyntaxNode<'_>) -> bool {
        matches!(&*node.kind(), "if_expression" | "if_let_expression")
    }

    fn calls(&self, node: &SyntaxNode<'_>, unit: &Unit<'_>) -> bool {
        if node.kind() != "call_expression" {
            return false;
        }
        let Some(callee) = node.field("function") else {
            return false;
        };
        match &*callee.kind() {
            "identifier" => callee.text() == unit.short,
            "scoped_identifier" => {
                text_of(callee.field("path")) == "Self"
                    && text_of(callee.field("name")) == unit.short
            }
            "field_expression" => {
                text_of(callee.field("value")) == "self"
                    && text_of(callee.field("field")) == unit.short
            }
            _ => false,
        }
    }
}

pub(crate) fn measure(source: &str) -> Vec<Function> {
    let grep = SupportLang::Rust.ast_grep(source);
    let root = grep.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return Vec::new();
    }
    let mut units = Vec::new();
    collect(&root, "", &mut units);
    structured::measure(&Rust, units)
}

fn qualified(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}::{name}")
    }
}

/// The functions of a file, a module, an `impl` or a `trait`.
fn collect<'r>(container: &SyntaxNode<'r>, prefix: &str, units: &mut Vec<Unit<'r>>) {
    for item in container.children() {
        match &*item.kind() {
            "function_item" => {
                let short = text_of(item.field("name"));
                units.push(Unit {
                    name: qualified(prefix, &short),
                    short,
                    receiver: None,
                    span: item.clone(),
                    function: item.clone(),
                });
            }
            "mod_item" => {
                if let Some(body) = item.field("body") {
                    collect(
                        &body,
                        &qualified(prefix, &text_of(item.field("name"))),
                        units,
                    );
                }
            }
            "impl_item" => {
                let implemented = text_of(item.field("type"));
                let name = implemented
                    .split('<')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .trim_start_matches('&')
                    .trim();
                if let Some(body) = item.field("body") {
                    collect(&body, &qualified(prefix, name), units);
                }
            }
            "trait_item" => {
                if let Some(body) = item.field("body") {
                    collect(
                        &body,
                        &qualified(prefix, &text_of(item.field("name"))),
                        units,
                    );
                }
            }
            _ => {}
        }
    }
}
