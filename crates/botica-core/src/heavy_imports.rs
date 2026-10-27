//! `no-eager-heavy-imports`: a heavy library imported when a Python module loads, outside a
//! function. An import inside `if TYPE_CHECKING:`, a `lazy import` of Python 3.15 and an import
//! that `__lazy_modules__` makes lazy are left alone.

use std::collections::BTreeSet;

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::native::{NativeRule, Needs};

pub const NO_EAGER_HEAVY_IMPORTS: &str = "no-eager-heavy-imports";

pub const DEFAULT_MODULES: [&str; 5] = ["torch", "tensorflow", "jax", "transformers", "cv2"];

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

#[derive(Debug)]
pub struct NoEagerHeavyImports {
    /// The modules that are heavy to load; a submodule of one is heavy too.
    pub modules: Vec<String>,
}

impl Default for NoEagerHeavyImports {
    fn default() -> Self {
        Self {
            modules: DEFAULT_MODULES
                .iter()
                .map(|&name| name.to_owned())
                .collect(),
        }
    }
}

/// A module-level `__lazy_modules__ = [...]`: where it ends and the names it lists.
struct LazyModules {
    end: usize,
    names: BTreeSet<String>,
}

impl NoEagerHeavyImports {
    fn is_heavy(&self, module: &str) -> bool {
        self.modules.iter().any(|heavy| {
            module == heavy
                || module
                    .strip_prefix(heavy.as_str())
                    .is_some_and(|rest| rest.starts_with('.'))
        })
    }
}

impl NativeRule for NoEagerHeavyImports {
    fn id(&self) -> &'static str {
        NO_EAGER_HEAVY_IMPORTS
    }

    fn needs(&self) -> Needs {
        Needs::Text
    }

    fn check(&self, doc: &Document<'_>, out: &mut Vec<Diagnostic>) {
        if self.modules.is_empty() {
            return;
        }
        let Some(text) = doc.text() else {
            return;
        };
        let grep = SupportLang::Python.ast_grep(text);
        let root = grep.root();
        let declarations = lazy_modules(&root);
        for statement in root.dfs().filter(|node| {
            matches!(
                node.kind().as_ref(),
                "import_statement" | "import_from_statement"
            )
        }) {
            if in_function(&statement)
                || under_type_checking(&statement)
                || after_lazy_keyword(&statement, text)
            {
                continue;
            }
            let can_be_lazy = !statement
                .ancestors()
                .any(|node| matches!(node.kind().as_ref(), "try_statement" | "class_definition"));
            for module in imported_modules(&statement) {
                let name = dotted(&module);
                if !self.is_heavy(&name) {
                    continue;
                }
                let declared = can_be_lazy
                    && declarations.iter().any(|declaration| {
                        declaration.end <= statement.range().start
                            && declaration.names.contains(&name)
                    });
                if declared {
                    continue;
                }
                out.push(finding(doc, &module, &name, &statement, can_be_lazy));
            }
        }
    }
}

fn finding(
    doc: &Document<'_>,
    module: &PyNode<'_>,
    name: &str,
    statement: &PyNode<'_>,
    can_be_lazy: bool,
) -> Diagnostic {
    let range = module.range();
    let (line, column) = doc.position(range.start);
    let (end_line, end_column) = doc.position(range.end);
    let lazy = if statement.kind() == "import_from_statement" {
        "lazy from"
    } else {
        "lazy import"
    };
    let fix = if can_be_lazy {
        format!(
            "import it inside the function that uses it, write `{lazy}` on Python 3.15 or later, \
             or name it in `__lazy_modules__` when the code also runs on older versions"
        )
    } else {
        "import it inside the function that uses it (an import in a `try` block or a class \
         body stays eager on Python 3.15)"
            .to_owned()
    };
    Diagnostic {
        path: doc.path.to_owned(),
        rule: NO_EAGER_HEAVY_IMPORTS.to_owned(),
        severity: Severity::Warning,
        start: range.start,
        end: range.end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message: format!("`{name}` is loaded when this module is imported, so every importer waits for it: {fix}"),
    }
}

/// The dotted names of the modules that an import statement loads. A relative import loads a
/// module of the project and gives none.
fn imported_modules<'r>(statement: &PyNode<'r>) -> Vec<PyNode<'r>> {
    if statement.kind() == "import_from_statement" {
        return statement
            .field("module_name")
            .filter(|module| module.kind() == "dotted_name")
            .into_iter()
            .collect();
    }
    statement
        .children()
        .filter_map(|child| match child.kind().as_ref() {
            "dotted_name" => Some(child),
            "aliased_import" => child.field("name"),
            _ => None,
        })
        .collect()
}

fn dotted(module: &PyNode<'_>) -> String {
    module
        .text()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn in_function(statement: &PyNode<'_>) -> bool {
    statement
        .ancestors()
        .any(|node| node.kind() == "function_definition")
}

/// Whether the statement is in the body of an `if TYPE_CHECKING:` (or `typing.TYPE_CHECKING`),
/// and not in its `else`.
fn under_type_checking(statement: &PyNode<'_>) -> bool {
    let mut child = statement.clone();
    for ancestor in statement.ancestors() {
        if ancestor.kind() == "if_statement" {
            let guards = ancestor.field("condition").is_some_and(|condition| {
                let condition = condition.text();
                condition == "TYPE_CHECKING" || condition.ends_with(".TYPE_CHECKING")
            });
            let in_body = ancestor
                .field("consequence")
                .is_some_and(|body| body.range() == child.range());
            if guards && in_body {
                return true;
            }
        }
        child = ancestor;
    }
    false
}

/// Whether the statement follows the soft keyword `lazy` of Python 3.15 (PEP 810). The grammar of
/// tree-sitter-python 0.25 does not know the keyword and parses it as an `ERROR` node with the
/// identifier `lazy`, right before the import statement on the same line.
fn after_lazy_keyword(statement: &PyNode<'_>, text: &str) -> bool {
    statement.prev().is_some_and(|previous| {
        previous.kind() == "ERROR"
            && previous.text().trim() == "lazy"
            && text[previous.range().end..statement.range().start]
                .chars()
                .all(|character| character == ' ' || character == '\t')
    })
}

/// The `__lazy_modules__` assignments at the top of the module, with the string literals of the
/// list, tuple or set on their right-hand side.
fn lazy_modules(root: &PyNode<'_>) -> Vec<LazyModules> {
    root.children()
        .filter(|statement| statement.kind() == "expression_statement")
        .filter_map(|statement| {
            let assignment = statement
                .children()
                .find(|child| child.kind() == "assignment")?;
            let target = assignment.field("left")?;
            if target.kind() != "identifier" || target.text() != "__lazy_modules__" {
                return None;
            }
            let value = assignment.field("right")?;
            let names = value
                .dfs()
                .filter(|node| node.kind() == "string_content")
                .map(|node| node.text().into_owned())
                .collect();
            Some(LazyModules {
                end: statement.range().end,
                names,
            })
        })
        .collect()
}
