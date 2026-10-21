//! The imports of a Python file: each `import` and `from … import` statement, wherever it is, with
//! the dotted names it imports.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::{LanguageExt, SupportLang};

type PyNode<'r> = Node<'r, StrDoc<SupportLang>>;

/// One import statement: its byte range, how many packages up a relative import starts (`0` for
/// an absolute one) and the dotted names it imports, from that package.
pub struct Import {
    pub start: usize,
    pub end: usize,
    pub level: usize,
    pub names: Vec<String>,
}

pub fn imports(text: &str) -> Vec<Import> {
    let grep = SupportLang::Python.ast_grep(text);
    grep.root()
        .dfs()
        .filter_map(|node| match &*node.kind() {
            "import_statement" => Some(plain(&node)),
            "import_from_statement" => from(&node),
            _ => None,
        })
        .collect()
}

/// The dotted name of a `dotted_name` or of the name of an `aliased_import`.
fn dotted(node: &PyNode<'_>) -> Option<String> {
    match &*node.kind() {
        "dotted_name" => Some(node.text().into_owned()),
        "aliased_import" => node.field("name").map(|name| name.text().into_owned()),
        _ => None,
    }
}

fn plain(node: &PyNode<'_>) -> Import {
    let range = node.range();
    Import {
        start: range.start,
        end: range.end,
        level: 0,
        names: node
            .field_children("name")
            .filter_map(|name| dotted(&name))
            .collect(),
    }
}

fn from(node: &PyNode<'_>) -> Option<Import> {
    let module = node.field("module_name")?;
    let (level, base) = if module.kind() == "relative_import" {
        let prefix = module
            .children()
            .find(|child| child.kind() == "import_prefix")
            .map_or(0, |prefix| {
                prefix.text().chars().filter(|&c| c == '.').count()
            });
        let base = module
            .children()
            .find(|child| child.kind() == "dotted_name")
            .map(|name| name.text().into_owned())
            .unwrap_or_default();
        (prefix, base)
    } else {
        (0, module.text().into_owned())
    };
    let mut names: Vec<String> = node
        .field_children("name")
        .filter_map(|name| dotted(&name))
        .map(|name| join(&base, &name))
        .collect();
    if node
        .children()
        .any(|child| child.kind() == "wildcard_import")
    {
        names.push(base);
    }
    let range = node.range();
    Some(Import {
        start: range.start,
        end: range.end,
        level,
        names,
    })
}

fn join(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_owned()
    } else {
        format!("{base}.{name}")
    }
}
