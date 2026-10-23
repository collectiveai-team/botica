---
id: CES-123
title: Doc comments follow the convention of their language and document what the code has
---

## Directive

1. The doc comment of a public function, type or module follows the convention of its language,
   in the table below.
2. A doc comment documents only what the code has. Each parameter it names is a parameter of the
   signature, a section of the return value belongs to a function that returns one, a section of
   yields belongs to a generator, and a link names an item that exists.
3. A doc comment says what the signature cannot say: what a parameter means, what the caller gets,
   when the function fails. The types stay in the signature.

| Language | Convention | Parts |
|---|---|---|
| Python | Google style docstrings ([PEP 257](https://peps.python.org/pep-0257/), [Google style](https://google.github.io/styleguide/pyguide.html#38-comments-and-docstrings)) | a summary line, then `Args:`, `Returns:`, `Yields:` and `Raises:` |
| TypeScript | [TSDoc](https://tsdoc.org/) in `/** */` | a summary, then `@param name - text`, `@returns`, `@throws`, with no type in a tag |
| JavaScript | [JSDoc](https://jsdoc.app/) in `/** */` | a summary, then `@param {Type} name`, `@returns`, `@yields`, `@throws` |
| Go | [Go doc comments](https://go.dev/doc/comment) | full sentences that start with the name of the item; a `Deprecated:` paragraph |
| Rust | [rustdoc](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html) in `///` | a summary line, then `# Errors`, `# Panics` and `# Safety`; links to items as `` [`Item`] `` |

## Why

A reader takes a doc comment at its word and reads the code less. A doc comment that names a
parameter the function lost, or a return value it no longer has, tells the next reader to pass the
parameter or to read the value. One convention per language lets the tools of that language read
the parts of a doc comment and compare them with the code.

## Tools

| Language | What checks the convention | What checks that the doc comment matches the code |
|---|---|---|
| Python | ruff, the `D` codes and `convention = "google"` of the section of `base-python` | ruff `DOC102`, `DOC202`, `DOC403`, through `docstring-matches-signature` |
| TypeScript, JavaScript | Biome `useSingleJsDocAsterisk`, which checks the asterisks of each line; eslint-plugin-jsdoc `check-tag-names`, which reports a tag that JSDoc and TSDoc do not define | eslint-plugin-jsdoc `check-param-names` and `require-returns-check`, in the hook `eslint-jsdoc` of `base-node`, through `docstring-matches-signature` |
| Go | golangci-lint `godoclint`, its `basic` rules (`pkg-doc`, `single-pkg-doc`, `start-with-name`, `deprecated`), in the `.golangci.yml` of `base-go` | `godoclint` `start-with-name`: the comment of a renamed function no longer starts with its name |
| Rust | clippy `missing_errors_doc` and `missing_panics_doc` at `warn`, `missing_safety_doc`, as flags of the hook of clippy of `base-rust` | rustdoc `broken_intra_doc_links` at `deny`, in the step of `cargo doc` of the workflow of `ci-rust` |

The hook of clippy of `base-rust` runs with `-D warnings`, so a lint at `warn` stops the commit like
one at `deny`. `cargo doc` writes the documentation of the whole workspace, so it runs in CI and not
in each commit. In TypeScript and JavaScript, Biome formats and lints the code, and ESLint runs
only the three rules of eslint-plugin-jsdoc: Biome has no rule that reads the tags of a JSDoc
comment.

## Examples

```python
def convert(amount: Decimal, rate: Decimal) -> Decimal:
    """Convert an amount at a rate.

    Args:
        amount: The amount, in the currency of the account.
        rate: The units of the target currency for one unit of the account.

    Returns:
        The amount in the target currency.

    """
```

```go
// Convert returns amount in the target currency at rate.
func Convert(amount, rate decimal.Decimal) decimal.Decimal
```

A docstring that keeps `currency:` under `Args:` after the parameter was removed breaks the
directive.

## Exceptions

A private function, a test and a generated file can go without a doc comment. When they have one,
it documents only what the code has.
