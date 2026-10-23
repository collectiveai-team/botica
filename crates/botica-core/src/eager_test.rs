//! `eager-test`: a test that calls more distinct functions of the code under test than the limit.
//! A test is a `test_*` function or method of a `Test*` class in Python, an `it` or `test` callback in
//! TypeScript and JavaScript, a `Test*` function in Go and a `#[test]` function in Rust.

use std::collections::BTreeSet;
use std::path::Path;

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{Language, LanguageExt, SupportLang};

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

pub const EAGER_TEST: &str = "eager-test";

type SyntaxNode<'r> = Node<'r, StrDoc<SupportLang>>;

#[derive(Debug)]
pub struct EagerTest {
    pub max: usize,
}

impl Default for EagerTest {
    fn default() -> Self {
        Self { max: 5 }
    }
}

/// A test: its name, the node that the finding covers and the node whose calls are counted.
struct Test<'r> {
    name: String,
    span: SyntaxNode<'r>,
    body: SyntaxNode<'r>,
}

/// Calls that check, arrange or convert, and not the code under test.
const PYTHON_OTHERS: [&str; 41] = [
    "abs",
    "all",
    "any",
    "approx",
    "bool",
    "call",
    "dict",
    "enumerate",
    "fail",
    "float",
    "frozenset",
    "getattr",
    "hasattr",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "list",
    "max",
    "min",
    "next",
    "param",
    "parametrize",
    "patch",
    "print",
    "range",
    "raises",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "skip",
    "sorted",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "warns",
];
const SCRIPT_ROOTS: [&str; 10] = [
    "expect", "assert", "vi", "jest", "console", "JSON", "Object", "Array", "Math", "Promise",
];
const SCRIPT_OTHERS: [&str; 10] = [
    "describe",
    "it",
    "test",
    "beforeEach",
    "afterEach",
    "beforeAll",
    "afterAll",
    "String",
    "Number",
    "Boolean",
];
const GO_PACKAGES: [&str; 7] = [
    "assert", "require", "fmt", "errors", "strings", "strconv", "testing",
];
const GO_OTHERS: [&str; 8] = [
    "append", "cap", "copy", "delete", "len", "make", "new", "panic",
];
const RUST_OTHERS: [&str; 26] = [
    "as_ref",
    "as_str",
    "clone",
    "collect",
    "default",
    "expect",
    "from",
    "into",
    "into_iter",
    "is_empty",
    "is_err",
    "is_none",
    "is_ok",
    "is_some",
    "iter",
    "len",
    "new",
    "to_owned",
    "to_string",
    "to_vec",
    "unwrap",
    "unwrap_err",
    "Some",
    "Ok",
    "Err",
    "Box",
];

fn text_of(node: Option<SyntaxNode<'_>>) -> String {
    node.map(|node| node.text().into_owned())
        .unwrap_or_default()
}

fn python_tests<'r>(root: &SyntaxNode<'r>) -> Vec<Test<'r>> {
    let mut tests = Vec::new();
    for item in root.children() {
        let definition = unwrap_decorated(item);
        match &*definition.kind() {
            "function_definition" => push_python(&mut tests, definition, ""),
            "class_definition" => {
                let class = text_of(definition.field("name"));
                let Some(body) = definition
                    .field("body")
                    .filter(|_| class.starts_with("Test"))
                else {
                    continue;
                };
                for member in body.children() {
                    let method = unwrap_decorated(member);
                    if method.kind() == "function_definition" {
                        push_python(&mut tests, method, &class);
                    }
                }
            }
            _ => {}
        }
    }
    tests
}

fn unwrap_decorated(node: SyntaxNode<'_>) -> SyntaxNode<'_> {
    if node.kind() == "decorated_definition" {
        node.field("definition").unwrap_or(node)
    } else {
        node
    }
}

fn push_python<'r>(tests: &mut Vec<Test<'r>>, function: SyntaxNode<'r>, class: &str) {
    let name = text_of(function.field("name"));
    let Some(body) = function.field("body").filter(|_| name.starts_with("test")) else {
        return;
    };
    let name = if class.is_empty() {
        name
    } else {
        format!("{class}::{name}")
    };
    tests.push(Test {
        name,
        span: function,
        body,
    });
}

fn python_callee(call: &SyntaxNode<'_>) -> Option<String> {
    let function = call.field("function")?;
    let name = match &*function.kind() {
        "identifier" => function.text().into_owned(),
        "attribute" => text_of(function.field("attribute")),
        _ => return None,
    };
    let other = name.starts_with("assert")
        || name.starts_with('_')
        || name.starts_with(|c: char| c.is_ascii_uppercase())
        || PYTHON_OTHERS.contains(&name.as_str());
    (!other).then_some(name)
}

fn script_tests<'r>(root: &SyntaxNode<'r>) -> Vec<Test<'r>> {
    root.dfs()
        .filter(|node| node.kind() == "call_expression")
        .filter(|call| {
            call.field("function")
                .is_some_and(|function| matches!(&*function.text(), "it" | "test"))
        })
        .filter_map(|call| {
            let arguments = call.field("arguments")?;
            let name = arguments
                .children()
                .find(|argument| matches!(&*argument.kind(), "string" | "template_string"))
                .map(|name| name.text().trim_matches(['"', '\'', '`']).to_owned())
                .unwrap_or_default();
            let callback = arguments.children().find(|argument| {
                matches!(
                    &*argument.kind(),
                    "arrow_function" | "function_expression" | "function"
                )
            })?;
            Some(Test {
                name,
                body: callback.field("body")?,
                span: call,
            })
        })
        .collect()
}

/// The identifier a chain of members and calls starts from: `expect` for `expect(x).not.toBe(y)`.
fn chain_root(node: SyntaxNode<'_>) -> String {
    let mut node = node;
    loop {
        let next = match &*node.kind() {
            "member_expression" => node.field("object"),
            "call_expression" => node.field("function"),
            _ => return node.text().into_owned(),
        };
        match next {
            Some(next) => node = next,
            None => return node.text().into_owned(),
        }
    }
}

fn script_callee(call: &SyntaxNode<'_>) -> Option<String> {
    let function = call.field("function")?;
    if SCRIPT_ROOTS.contains(&chain_root(function.clone()).as_str()) {
        return None;
    }
    let name = match &*function.kind() {
        "identifier" => function.text().into_owned(),
        "member_expression" => text_of(function.field("property")),
        _ => return None,
    };
    (!SCRIPT_OTHERS.contains(&name.as_str())).then_some(name)
}

fn go_tests<'r>(root: &SyntaxNode<'r>) -> Vec<Test<'r>> {
    root.children()
        .filter(|node| node.kind() == "function_declaration")
        .filter_map(|function| {
            let name = text_of(function.field("name"));
            let body = function
                .field("body")
                .filter(|_| name.starts_with("Test"))?;
            Some(Test {
                name,
                body,
                span: function,
            })
        })
        .collect()
}

/// The name of the `*testing.T` parameter of a Go test, `t` most of the time.
fn go_receiver(test: &Test<'_>) -> String {
    test.span
        .field("parameters")
        .and_then(|list| {
            list.children()
                .find(|child| child.kind() == "parameter_declaration")
        })
        .and_then(|parameter| parameter.field("name"))
        .map(|name| name.text().into_owned())
        .unwrap_or_default()
}

fn go_callee(call: &SyntaxNode<'_>, receiver: &str) -> Option<String> {
    let function = call.field("function")?;
    match &*function.kind() {
        "identifier" => {
            let name = function.text().into_owned();
            (!GO_OTHERS.contains(&name.as_str())).then_some(name)
        }
        "selector_expression" => {
            let operand = text_of(function.field("operand"));
            if operand == receiver || GO_PACKAGES.contains(&operand.as_str()) {
                return None;
            }
            Some(text_of(function.field("field")))
        }
        _ => None,
    }
}

fn rust_tests<'r>(root: &SyntaxNode<'r>) -> Vec<Test<'r>> {
    root.dfs()
        .filter(|node| node.kind() == "function_item")
        .filter(|function| {
            function
                .prev_all()
                .take_while(|sibling| sibling.kind() == "attribute_item")
                .any(|attribute| {
                    let text = attribute.text();
                    text == "#[test]" || text.ends_with("::test]")
                })
        })
        .filter_map(|function| {
            Some(Test {
                name: text_of(function.field("name")),
                body: function.field("body")?,
                span: function,
            })
        })
        .collect()
}

fn rust_callee(call: &SyntaxNode<'_>) -> Option<String> {
    let function = call.field("function")?;
    let name = match &*function.kind() {
        "identifier" => function.text().into_owned(),
        "scoped_identifier" => text_of(function.field("name")),
        "field_expression" => text_of(function.field("field")),
        _ => return None,
    };
    (!RUST_OTHERS.contains(&name.as_str())).then_some(name)
}

/// The distinct functions of the code under test that each test under `root` calls.
fn calls_per_test<'r>(
    language: SupportLang,
    root: &SyntaxNode<'r>,
) -> Vec<(Test<'r>, BTreeSet<String>)> {
    let tests = match language {
        SupportLang::Python => python_tests(root),
        SupportLang::TypeScript | SupportLang::Tsx | SupportLang::JavaScript => script_tests(root),
        SupportLang::Go => go_tests(root),
        SupportLang::Rust => rust_tests(root),
        _ => Vec::new(),
    };
    tests
        .into_iter()
        .map(|test| {
            let receiver = go_receiver(&test);
            let names = test
                .body
                .dfs()
                .filter_map(|node| match (language, &*node.kind()) {
                    (SupportLang::Python, "call") => python_callee(&node),
                    (SupportLang::Go, "call_expression") => go_callee(&node, &receiver),
                    (SupportLang::Rust, "call_expression") => rust_callee(&node),
                    (
                        SupportLang::TypeScript | SupportLang::Tsx | SupportLang::JavaScript,
                        "call_expression",
                    ) => script_callee(&node),
                    _ => None,
                })
                .collect();
            (test, names)
        })
        .collect()
}

impl NativeRule for EagerTest {
    fn id(&self) -> &'static str {
        EAGER_TEST
    }

    fn needs(&self) -> Needs {
        Needs::Text
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        let (Some(text), Some(language)) =
            (doc.text(), SupportLang::from_path(Path::new(doc.path)))
        else {
            return;
        };
        let grep = language.ast_grep(text);
        for (test, names) in calls_per_test(language, &grep.root()) {
            if names.len() <= self.max {
                continue;
            }
            let range = test.span.range();
            let (line, column) = doc.position(range.start);
            let (end_line, end_column) = doc.position(range.end);
            let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
            out.push(Diagnostic {
                path: doc.path.to_owned(),
                rule: EAGER_TEST.to_owned(),
                severity: Severity::Warning,
                start: range.start,
                end: range.end,
                line,
                column,
                end_line,
                end_column,
                file_level: false,
                message: format!(
                    "`{}` calls {} functions of the code under test ({}; the limit is {}): split it \
                     into tests that each check one behaviour",
                    test.name,
                    names.len(),
                    listed.join(", "),
                    self.max
                ),
                measure: None,
            });
        }
    }
}
