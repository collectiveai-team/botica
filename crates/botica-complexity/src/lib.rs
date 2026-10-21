//! The cognitive complexity of the functions of a source file, as the paper of SonarSource defines
//! it. A function nested in another adds to the one that holds it, as complexipy counts it.

use ast_grep_language::SupportLang;

mod go;
mod python;
mod rust;
mod structured;
mod typescript;

/// A function that is measured on its own, with what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The names of the function and of what holds it, joined with `::`: `Order::total`.
    pub name: String,
    /// The byte offset where the definition starts, after its decorators or attributes.
    pub start: usize,
    /// The byte offset where the definition ends.
    pub end: usize,
    pub complexity: u32,
}

/// The functions of `source` in the order they appear. A language that the crate does not measure,
/// or a source that does not parse, gives none.
pub fn measure(language: SupportLang, source: &str) -> Vec<Function> {
    match language {
        SupportLang::Python => python::measure(source),
        SupportLang::TypeScript | SupportLang::Tsx => typescript::measure(language, source),
        SupportLang::Go => go::measure(source),
        SupportLang::Rust => rust::measure(source),
        _ => Vec::new(),
    }
}
