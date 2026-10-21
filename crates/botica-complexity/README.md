# botica-complexity

`measure` takes a language and a source text and returns each function that is measured on its
own, with its qualified name, its range and its cognitive complexity
([ADR 0011](../../docs/adr/0011-native-cognitive-complexity.md)). It parses with the tree-sitter
grammars of `ast-grep-language` and knows nothing of rules, findings or configuration; the rule
`cognitive-complexity` of `botica-core` uses it.

## complexipy

The count follows the paper
[Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) of SonarSource.
For Python it follows [complexipy](https://github.com/rohaquinlop/complexipy) 8.1.0, by Robin
Quintero, under the MIT license: `src/python/` ports `count_bool_ops`,
`count_different_childs_type` and `statement_cognitive_complexity_shared` of its
`crates/complexipy-core`, and a function nested in another adds to the one that holds it, as
complexipy counts it.

`tests/fixtures/complexipy/src` holds the files of `tests/src` of complexipy 8.1.0, with its
license in `tests/fixtures/complexipy/LICENSE`. `expected.json` holds the value of each function
that `complexipy tests/src --no-ignore --output-format json` gives, and
`tests/complexipy_parity.rs` checks them. `tests/python.rs` holds the cases of
`TestPaperConformance` of its `tests/main.py`.

A value that differs from the one of complexipy is a bug, unless the table below lists it with its
reason; its test then expects the value of botica. A later release of complexipy is taken in a
change of its own: copy its `tests/src`, write `expected.json` again and read the changes of its
`cognitive_complexity.rs`.

| Difference | Reason |
| --- | --- |
| The code at the top of a module is not measured. complexipy adds it to the total of the file. | botica reports functions (ADR 0011, decision 3). |
| `# complexipy: ignore` and `# noqa: complexipy` leave a function measured. | The rule is silenced with the comments of botica (ADR 0011, decision 7). |

## TypeScript, Go and Rust

`src/structured.rs` holds the count for the three, and each language says what a kind of node is.
They have no reference to match: `tests/braces.rs` writes the examples of the paper in each
language.
