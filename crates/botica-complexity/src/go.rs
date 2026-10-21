//! Go.

use ast_grep_language::{LanguageExt, SupportLang};

use crate::structured::{self, text_of, Grammar, Role, SyntaxNode, Unit};
use crate::Function;

struct Go;

const CASES: [&str; 4] = [
    "expression_case",
    "default_case",
    "type_case",
    "communication_case",
];

impl Grammar for Go {
    fn role<'r>(&self, node: &SyntaxNode<'r>) -> Role<'r> {
        match &*node.kind() {
            "if_statement" => Role::If {
                header: node
                    .field("initializer")
                    .into_iter()
                    .chain(node.field("condition"))
                    .collect(),
                consequence: node.field("consequence"),
                alternative: node.field("alternative"),
            },
            "for_statement" => Role::Structural {
                nested: node.field("body").into_iter().collect(),
            },
            "expression_switch_statement" | "type_switch_statement" | "select_statement" => {
                Role::Structural {
                    nested: node
                        .children()
                        .filter(|child| CASES.contains(&&*child.kind()))
                        .collect(),
                }
            }
            "binary_expression" if self.logical_operator(node).is_some() => Role::Logical,
            "break_statement" | "continue_statement"
                if node.children().any(|child| child.kind() == "label_name") =>
            {
                Role::Flat
            }
            "goto_statement" => Role::Flat,
            "func_literal" | "function_declaration" | "method_declaration" => Role::Function,
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
        node.kind() == "if_statement"
    }

    fn calls(&self, node: &SyntaxNode<'_>, unit: &Unit<'_>) -> bool {
        if node.kind() != "call_expression" {
            return false;
        }
        let Some(callee) = node.field("function") else {
            return false;
        };
        match &*callee.kind() {
            "identifier" => unit.receiver.is_none() && callee.text() == unit.short,
            "selector_expression" => {
                unit.receiver
                    .as_deref()
                    .is_some_and(|receiver| text_of(callee.field("operand")) == receiver)
                    && text_of(callee.field("field")) == unit.short
            }
            _ => false,
        }
    }
}

pub(crate) fn measure(source: &str) -> Vec<Function> {
    let grep = SupportLang::Go.ast_grep(source);
    let root = grep.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return Vec::new();
    }
    let mut units = Vec::new();
    for declaration in root.children() {
        let short = text_of(declaration.field("name"));
        match &*declaration.kind() {
            "function_declaration" => units.push(Unit {
                name: short.clone(),
                short,
                receiver: None,
                span: declaration.clone(),
                function: declaration.clone(),
            }),
            "method_declaration" => {
                let receiver = declaration.field("receiver").and_then(|list| {
                    list.children()
                        .find(|child| child.kind() == "parameter_declaration")
                });
                let receiver_type = receiver
                    .as_ref()
                    .and_then(|parameter| {
                        parameter
                            .field("type")?
                            .dfs()
                            .find(|node| node.kind() == "type_identifier")
                    })
                    .map(|node| node.text().into_owned())
                    .unwrap_or_default();
                let variable = receiver
                    .and_then(|parameter| parameter.field("name"))
                    .map(|name| name.text().into_owned());
                units.push(Unit {
                    name: format!("{receiver_type}::{short}"),
                    short,
                    receiver: variable,
                    span: declaration.clone(),
                    function: declaration.clone(),
                });
            }
            _ => {}
        }
    }
    structured::measure(&Go, units)
}
