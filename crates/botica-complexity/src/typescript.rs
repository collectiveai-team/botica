//! TypeScript and TSX.

use ast_grep_language::{LanguageExt, SupportLang};

use crate::structured::{self, text_of, Grammar, Role, SyntaxNode, Unit};
use crate::Function;

struct TypeScript;

const FUNCTIONS: [&str; 7] = [
    "function_declaration",
    "generator_function_declaration",
    "function_expression",
    "function",
    "generator_function",
    "arrow_function",
    "method_definition",
];

const VALUES: [&str; 4] = [
    "arrow_function",
    "function_expression",
    "function",
    "generator_function",
];

impl Grammar for TypeScript {
    fn role<'r>(&self, node: &SyntaxNode<'r>) -> Role<'r> {
        let kind = node.kind();
        match &*kind {
            "if_statement" => Role::If {
                header: node.field("condition").into_iter().collect(),
                consequence: node.field("consequence"),
                alternative: node.field("alternative"),
            },
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
            | "switch_statement" | "catch_clause" => Role::Structural {
                nested: node.field("body").into_iter().collect(),
            },
            "ternary_expression" => Role::Structural {
                nested: node
                    .field("consequence")
                    .into_iter()
                    .chain(node.field("alternative"))
                    .collect(),
            },
            "binary_expression" if self.logical_operator(node).is_some() => Role::Logical,
            "break_statement" | "continue_statement" if node.field("label").is_some() => Role::Flat,
            _ if FUNCTIONS.contains(&&*kind) => Role::Function,
            _ => Role::Plain,
        }
    }

    fn logical_operator(&self, node: &SyntaxNode<'_>) -> Option<String> {
        if node.kind() != "binary_expression" {
            return None;
        }
        let operator = text_of(node.field("operator"));
        matches!(operator.as_str(), "&&" | "||" | "??").then_some(operator)
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
            "identifier" => callee.text() == unit.short,
            "member_expression" => {
                callee
                    .field("object")
                    .is_some_and(|object| object.kind() == "this")
                    && text_of(callee.field("property")) == unit.short
            }
            _ => false,
        }
    }
}

pub(crate) fn measure(language: SupportLang, source: &str) -> Vec<Function> {
    let grep = language.ast_grep(source);
    let root = grep.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return Vec::new();
    }
    let mut units = Vec::new();
    for statement in root.children() {
        let declaration = if statement.kind() == "export_statement" {
            match statement
                .field("declaration")
                .or_else(|| statement.field("value"))
            {
                Some(declaration) => declaration,
                None => continue,
            }
        } else {
            statement
        };
        collect(&declaration, &mut units);
    }
    structured::measure(&TypeScript, units)
}

fn collect<'r>(declaration: &SyntaxNode<'r>, units: &mut Vec<Unit<'r>>) {
    match &*declaration.kind() {
        "function_declaration"
        | "generator_function_declaration"
        | "function_expression"
        | "function" => {
            let name = text_of(declaration.field("name"));
            let name = if name.is_empty() {
                "default".to_owned()
            } else {
                name
            };
            units.push(unit(name.clone(), name, declaration, declaration));
        }
        "lexical_declaration" | "variable_declaration" => {
            for declarator in declaration
                .children()
                .filter(|child| child.kind() == "variable_declarator")
            {
                if let Some(value) = declarator
                    .field("value")
                    .filter(|value| VALUES.contains(&&*value.kind()))
                {
                    let name = text_of(declarator.field("name"));
                    units.push(unit(name.clone(), name, &declarator, &value));
                }
            }
        }
        "class_declaration" | "abstract_class_declaration" | "class" => {
            let class = text_of(declaration.field("name"));
            let Some(body) = declaration.field("body") else {
                return;
            };
            for member in body.children() {
                let short = text_of(member.field("name"));
                match &*member.kind() {
                    "method_definition" => {
                        units.push(unit(format!("{class}::{short}"), short, &member, &member));
                    }
                    "public_field_definition" => {
                        if let Some(value) = member
                            .field("value")
                            .filter(|value| VALUES.contains(&&*value.kind()))
                        {
                            units.push(unit(format!("{class}::{short}"), short, &member, &value));
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn unit<'r>(
    name: String,
    short: String,
    span: &SyntaxNode<'r>,
    function: &SyntaxNode<'r>,
) -> Unit<'r> {
    Unit {
        name,
        short,
        receiver: None,
        span: span.clone(),
        function: function.clone(),
    }
}
