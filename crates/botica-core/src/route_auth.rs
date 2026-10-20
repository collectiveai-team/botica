//! `route-auth`: each route of a FastAPI application gets its caller from one of the dependencies
//! the configuration names, or the configuration lists it as public. The rule reads one file: a
//! route is guarded by its parameters, by the `dependencies` of its decorator, or by those of the
//! `APIRouter` or `FastAPI` object of the same file that the decorator uses.

use std::collections::HashSet;

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

pub const ROUTE_AUTH: &str = "route-auth";

/// The methods of `FastAPI` and `APIRouter` that declare a route.
const ROUTE_METHODS: [&str; 10] = [
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "options",
    "head",
    "trace",
    "api_route",
    "websocket",
];

/// The calls that declare a dependency.
const MARKERS: [&str; 2] = ["Depends", "Security"];

/// The calls that make an object routes are declared on.
const ROUTERS: [&str; 2] = ["APIRouter", "FastAPI"];

#[derive(Debug, Default)]
pub struct RouteAuth {
    /// The dependencies that verify the caller, or the `Annotated` aliases that carry one.
    pub dependencies: Vec<String>,
    /// The names of the route functions that need no caller.
    pub public: Vec<String>,
}

/// The last part of a dotted name: `fastapi.Depends` is `Depends`.
fn last_part(text: &str) -> &str {
    text.rsplit('.').next().unwrap_or(text)
}

impl RouteAuth {
    fn names(&self, text: &str) -> bool {
        self.dependencies
            .iter()
            .any(|name| text == name || last_part(text) == last_part(name))
    }

    /// Whether `node` holds a `Depends(...)` or `Security(...)` of one of the dependencies.
    fn has_guard(&self, node: &PyNode<'_>) -> bool {
        node.dfs().any(|call| {
            if call.kind() != "call" {
                return false;
            }
            let Some(function) = call.field("function") else {
                return false;
            };
            if !MARKERS.contains(&last_part(&function.text())) {
                return false;
            }
            call.field("arguments")
                .and_then(|arguments| {
                    arguments
                        .children()
                        .find(|argument| match &*argument.kind() {
                            "keyword_argument" => argument
                                .field("name")
                                .is_some_and(|name| name.text() == "dependency"),
                            "(" | ")" | "," | "comment" => false,
                            _ => true,
                        })
                })
                .map(|argument| match &*argument.kind() {
                    "keyword_argument" => argument
                        .field("value")
                        .map(|value| value.text().into_owned())
                        .unwrap_or_default(),
                    _ => argument.text().into_owned(),
                })
                .is_some_and(|name| self.names(&name))
        })
    }

    /// Whether the parameters of a route name a dependency or an alias that carries one.
    fn guarded_parameters(&self, parameters: &PyNode<'_>, aliases: &HashSet<String>) -> bool {
        self.has_guard(parameters)
            || parameters.dfs().any(|node| {
                matches!(&*node.kind(), "identifier" | "attribute")
                    && (aliases.contains(&*node.text()) || self.names(&node.text()))
                    && node.parent().is_some_and(|parent| {
                        parent.kind() == "type" || parent.kind() == "subscript"
                    })
            })
    }

    /// The routers of the module whose `dependencies` hold a guard, and the aliases that hold one.
    fn module_guards(&self, root: &PyNode<'_>) -> (HashSet<String>, HashSet<String>) {
        let mut routers = HashSet::new();
        let mut aliases = HashSet::new();
        for statement in root.children() {
            let (name, value) = match &*statement.kind() {
                "expression_statement" => {
                    let Some(assignment) = statement
                        .children()
                        .find(|child| child.kind() == "assignment")
                    else {
                        continue;
                    };
                    match (assignment.field("left"), assignment.field("right")) {
                        (Some(left), Some(right)) => (left.text().into_owned(), right),
                        _ => continue,
                    }
                }
                "type_alias_statement" => {
                    match (statement.field("left"), statement.field("right")) {
                        (Some(left), Some(right)) => (left.text().into_owned(), right),
                        _ => continue,
                    }
                }
                _ => continue,
            };
            if !self.has_guard(&value) {
                continue;
            }
            let makes_router = value.kind() == "call"
                && value
                    .field("function")
                    .is_some_and(|function| ROUTERS.contains(&last_part(&function.text())));
            if makes_router {
                routers.insert(name);
            } else {
                aliases.insert(name);
            }
        }
        (routers, aliases)
    }
}

/// The object of a route decorator, `app` in `@app.get("/")`, when the decorator declares one.
fn route_object(decorator: &PyNode<'_>) -> Option<String> {
    let call = decorator.children().find(|child| child.kind() == "call")?;
    let function = call.field("function")?;
    if function.kind() != "attribute" {
        return None;
    }
    let method = function.field("attribute")?;
    if !ROUTE_METHODS.contains(&&*method.text()) {
        return None;
    }
    Some(function.field("object")?.text().into_owned())
}

impl NativeRule for RouteAuth {
    fn id(&self) -> &'static str {
        ROUTE_AUTH
    }

    fn needs(&self) -> Needs {
        Needs::Text
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        if self.dependencies.is_empty() {
            return;
        }
        let Some(text) = doc.text() else {
            return;
        };
        let grep = SupportLang::Python.ast_grep(text);
        let root = grep.root();
        let (routers, aliases) = self.module_guards(&root);
        for definition in root
            .dfs()
            .filter(|node| node.kind() == "decorated_definition")
        {
            let Some(function) = definition
                .field("definition")
                .filter(|node| node.kind() == "function_definition")
            else {
                continue;
            };
            let routes: Vec<(PyNode<'_>, String)> = definition
                .children()
                .filter(|child| child.kind() == "decorator")
                .filter_map(|decorator| route_object(&decorator).map(|object| (decorator, object)))
                .collect();
            let Some((first, _)) = routes.first() else {
                continue;
            };
            let name = function
                .field("name")
                .map(|name| name.text().into_owned())
                .unwrap_or_default();
            if self.public.contains(&name) {
                continue;
            }
            let guarded = routes
                .iter()
                .any(|(decorator, object)| routers.contains(object) || self.has_guard(decorator))
                || function
                    .field("parameters")
                    .is_some_and(|parameters| self.guarded_parameters(&parameters, &aliases));
            if guarded {
                continue;
            }
            let range = first.range();
            let (line, column) = doc.position(range.start);
            let (end_line, end_column) = doc.position(range.end);
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: ROUTE_AUTH.to_owned(),
                severity: Severity::Error,
                start: range.start,
                end: range.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: format!(
                    "The route `{name}` gets no caller from a dependency that verifies a credential: \
                     add one of the `dependencies` of `[rules.route-auth]` to its parameters, to the \
                     `dependencies` of its decorator or of its router, or list `{name}` in `public`"
                ),
            });
        }
    }
}
