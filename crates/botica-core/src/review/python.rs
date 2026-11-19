use std::collections::BTreeSet;

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

fn imported_constructors(statement: &PyNode<'_>) -> Vec<String> {
    if statement.kind() != "import_from_statement" {
        return Vec::new();
    }
    statement
        .text()
        .strip_prefix("from fastapi import ")
        .map(|imports| {
            imports
                .split(',')
                .map(str::trim)
                .filter(|name| matches!(*name, "FastAPI" | "APIRouter"))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn assigned_router(
    statement: &PyNode<'_>,
    constructors: &BTreeSet<String>,
) -> Option<(String, bool)> {
    if statement.kind() != "expression_statement" {
        return None;
    }
    let assignment = statement
        .children()
        .find(|node| node.kind() == "assignment")?;
    let left = assignment.field("left")?;
    let right = assignment.field("right")?;
    let constructed = right.kind() == "call"
        && right
            .field("function")
            .is_some_and(|node| constructors.contains(node.text().as_ref()));
    Some((left.text().into_owned(), constructed))
}

fn decorated_route(decorator: &PyNode<'_>, routers: &BTreeSet<String>) -> Option<String> {
    let call = decorator.children().find(|node| node.kind() == "call")?;
    let function = call.field("function")?;
    if function.kind() != "attribute" {
        return None;
    }
    let object = function.field("object")?;
    let method = function.field("attribute")?;
    if !routers.contains(object.text().as_ref())
        || ![
            "get",
            "post",
            "put",
            "patch",
            "delete",
            "options",
            "head",
            "trace",
            "websocket",
        ]
        .contains(&method.text().as_ref())
    {
        return None;
    }
    let arguments = call.field("arguments")?;
    let path = arguments.children().find(|node| node.kind() == "string")?;
    let literal = path.text();
    if !literal.starts_with(['\'', '"'])
        || literal.contains('\\')
        || path.dfs().any(|node| node.kind() == "interpolation")
    {
        return None;
    }
    Some(format!(
        "{} {}",
        method.text(),
        literal.trim_matches(['\'', '"'])
    ))
}

/// Static, top-level FastAPI decorators with literal paths and directly imported constructors.
pub(super) fn routes(text: Option<&str>) -> BTreeSet<String> {
    let Some(text) = text else {
        return BTreeSet::new();
    };
    let grep = SupportLang::Python.ast_grep(text);
    let root = grep.root();
    let mut constructors = BTreeSet::new();
    let mut routers = BTreeSet::new();
    let mut routes = BTreeSet::new();
    for statement in root.children() {
        constructors.extend(imported_constructors(&statement));
        if let Some((name, constructed)) = assigned_router(&statement, &constructors) {
            routers.remove(&name);
            if constructed {
                routers.insert(name);
            }
        }
        if statement.kind() != "decorated_definition" {
            continue;
        }
        routes.extend(
            statement
                .children()
                .filter(|node| node.kind() == "decorator")
                .filter_map(|node| decorated_route(&node, &routers)),
        );
    }
    routes
}

pub(super) fn assertions(text: Option<&str>) -> BTreeSet<String> {
    let Some(text) = text else {
        return BTreeSet::new();
    };
    let grep = SupportLang::Python.ast_grep(text);
    grep.root()
        .dfs()
        .filter(|node| node.kind() == "assert_statement")
        .map(|node| node.text().into_owned())
        .collect()
}
