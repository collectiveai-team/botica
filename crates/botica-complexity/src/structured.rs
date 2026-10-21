//! The count of the paper for the languages with braces: TypeScript, Go and Rust. Each language
//! says what a kind of node is, and which functions are measured on their own.

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::Node;
use ast_grep_language::SupportLang;

use crate::Function;

pub(crate) type SyntaxNode<'r> = Node<'r, StrDoc<SupportLang>>;

/// What a node adds.
pub(crate) enum Role<'r> {
    /// An `if`: one plus the nesting for the first of a chain, one for each `else if` and `else`.
    If {
        header: Vec<SyntaxNode<'r>>,
        consequence: Option<SyntaxNode<'r>>,
        alternative: Option<SyntaxNode<'r>>,
    },
    /// A loop, a `switch`, `match` or `select`, a `catch` or a ternary: one plus the nesting.
    /// `nested` is one level deeper; the other children stay at the same level.
    Structural {
        nested: Vec<SyntaxNode<'r>>,
    },
    /// A binary operator that starts a sequence of `&&`, `||` or `??`.
    Logical,
    /// A function, lambda or closure: one level deeper, with no increment.
    Function,
    /// One, with no nesting: a `break` or `continue` with a label, a `goto`.
    Flat,
    Plain,
}

/// A function that is measured on its own.
pub(crate) struct Unit<'r> {
    pub name: String,
    /// The name that a recursive call uses.
    pub short: String,
    /// The variable of the receiver of a Go method.
    pub receiver: Option<String>,
    /// What the range of the finding covers.
    pub span: SyntaxNode<'r>,
    /// The function whose children are measured.
    pub function: SyntaxNode<'r>,
}

pub(crate) trait Grammar {
    fn role<'r>(&self, node: &SyntaxNode<'r>) -> Role<'r>;

    /// The operator of a binary node, when it is `&&`, `||` or `??`.
    fn logical_operator(&self, node: &SyntaxNode<'_>) -> Option<String>;

    fn is_if(&self, node: &SyntaxNode<'_>) -> bool;

    /// Whether `node` calls `unit` by its name.
    fn calls(&self, node: &SyntaxNode<'_>, unit: &Unit<'_>) -> bool;
}

pub(crate) fn measure<G: Grammar>(grammar: &G, units: Vec<Unit<'_>>) -> Vec<Function> {
    units
        .into_iter()
        .map(|unit| {
            let mut complexity: u32 = unit
                .function
                .children()
                .map(|child| walk(grammar, &child, 0))
                .sum();
            if calls_itself(grammar, &unit) {
                complexity += 1;
            }
            let range = unit.span.range();
            Function {
                name: unit.name,
                start: range.start,
                end: range.end,
                complexity,
            }
        })
        .collect()
}

fn walk<G: Grammar>(grammar: &G, node: &SyntaxNode<'_>, nesting: u32) -> u32 {
    match grammar.role(node) {
        Role::If { .. } => if_chain(grammar, node, nesting, true),
        Role::Structural { nested } => {
            let ids: Vec<usize> = nested.iter().map(SyntaxNode::node_id).collect();
            let header: u32 = node
                .children()
                .filter(|child| !ids.contains(&child.node_id()))
                .map(|child| walk(grammar, &child, nesting))
                .sum();
            let body: u32 = nested
                .iter()
                .map(|child| walk(grammar, child, nesting + 1))
                .sum();
            1 + nesting + header + body
        }
        Role::Logical => {
            let mut operators = Vec::new();
            let mut operands = Vec::new();
            flatten(grammar, node, &mut operators, &mut operands);
            let sequences = operators
                .iter()
                .enumerate()
                .filter(|(index, operator)| *index == 0 || operators[index - 1] != **operator)
                .count();
            let sequences = u32::try_from(sequences).unwrap_or(u32::MAX);
            sequences
                + operands
                    .iter()
                    .map(|operand| walk(grammar, operand, nesting))
                    .sum::<u32>()
        }
        Role::Function => children(grammar, node, nesting + 1),
        Role::Flat => 1 + children(grammar, node, nesting),
        Role::Plain => children(grammar, node, nesting),
    }
}

fn children<G: Grammar>(grammar: &G, node: &SyntaxNode<'_>, nesting: u32) -> u32 {
    node.children()
        .map(|child| walk(grammar, &child, nesting))
        .sum()
}

fn if_chain<G: Grammar>(grammar: &G, node: &SyntaxNode<'_>, nesting: u32, first: bool) -> u32 {
    let Role::If {
        header,
        consequence,
        alternative,
    } = grammar.role(node)
    else {
        return walk(grammar, node, nesting);
    };
    let mut complexity = if first { 1 + nesting } else { 1 };
    complexity += header
        .iter()
        .map(|part| walk(grammar, part, nesting))
        .sum::<u32>();
    complexity += consequence.map_or(0, |block| walk(grammar, &block, nesting + 1));
    if let Some(alternative) = alternative {
        let inner = if alternative.kind() == "else_clause" {
            first_named(&alternative)
        } else {
            None
        };
        let branch = inner.unwrap_or(alternative);
        complexity += if grammar.is_if(&branch) {
            if_chain(grammar, &branch, nesting, false)
        } else {
            1 + walk(grammar, &branch, nesting + 1)
        };
    }
    complexity
}

fn flatten<'r, G: Grammar>(
    grammar: &G,
    node: &SyntaxNode<'r>,
    operators: &mut Vec<String>,
    operands: &mut Vec<SyntaxNode<'r>>,
) {
    let inner = unwrap_parentheses(node);
    match grammar.logical_operator(&inner) {
        Some(operator) => {
            if let Some(left) = inner.field("left") {
                flatten(grammar, &left, operators, operands);
            }
            operators.push(operator);
            if let Some(right) = inner.field("right") {
                flatten(grammar, &right, operators, operands);
            }
        }
        None => operands.push(node.clone()),
    }
}

fn unwrap_parentheses<'r>(node: &SyntaxNode<'r>) -> SyntaxNode<'r> {
    let mut node = node.clone();
    while node.kind() == "parenthesized_expression" {
        match first_named(&node) {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

fn first_named<'r>(node: &SyntaxNode<'r>) -> Option<SyntaxNode<'r>> {
    let found = node
        .children()
        .find(|child| child.is_named() && child.kind() != "comment");
    found
}

/// Whether the body of `unit` calls it, outside the functions, lambdas and closures it holds.
fn calls_itself<G: Grammar>(grammar: &G, unit: &Unit<'_>) -> bool {
    let mut pending: Vec<SyntaxNode<'_>> = unit.function.children().collect();
    while let Some(node) = pending.pop() {
        if matches!(grammar.role(&node), Role::Function) {
            continue;
        }
        if grammar.calls(&node, unit) {
            return true;
        }
        pending.extend(node.children());
    }
    false
}

pub(crate) fn text_of(node: Option<SyntaxNode<'_>>) -> String {
    node.map(|node| node.text().into_owned())
        .unwrap_or_default()
}
