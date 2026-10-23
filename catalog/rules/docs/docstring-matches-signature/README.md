## Directive

A doc comment documents only what the function has. In Python, each name under `Args:` of a Google
style docstring is a parameter of the signature, a `Returns:` section belongs to a function that
returns a value, and a `Yields:` section belongs to a generator. In TypeScript and JavaScript, each
`@param` of a JSDoc comment names a parameter of the function, `@returns` belongs to a function that
returns a value, and each tag is one that JSDoc or TSDoc defines.

## Why

A reader takes a doc comment at its word. A doc comment that documents a parameter the function no
longer has tells the next reader, or the next agent, to pass it. `D417` of ruff reports a parameter
that a docstring leaves out; `DOC102`, `DOC202` and `DOC403` report the parameter, the return value
and the yields that it documents and the function does not have. eslint-plugin-jsdoc does the same
for a JSDoc comment, and also reports a tag with a typo, such as `@params`, that a reader of the
comment and the tools that read it skip.

## Tool

In Python, [ruff](https://docs.astral.sh/ruff/rules/#pydoclint-doc) checks the code through the
hook `ruff-check` of `prek.toml`. The three codes are preview rules of ruff 0.17.1, so they run only
with `preview = true`. `explicit-preview-rules = true` turns on a preview rule only when `select`
names its full code, so `E`, `RUF` or `DOC` turn on no preview rule, and the three codes are
written in full. Both settings go in `[tool.ruff.lint]`: in `[tool.ruff]`, `preview` also changes
the style of `ruff format`. `external-config` reports a `pyproject.toml` that lacks one of these
settings, one of the three codes in `select` or the Google convention.

In TypeScript and JavaScript, Biome formats and lints the code, and no rule of Biome reads the tags
of a JSDoc comment. ESLint runs three rules of
[eslint-plugin-jsdoc](https://github.com/gajus/eslint-plugin-jsdoc#rules) and no other rule, in the
hook `eslint-jsdoc` of `prek.toml` that the recipe `base-node` adds with its `eslint.config.js`:
`check-param-names`, `require-returns-check` and `check-tag-names`. `@typescript-eslint/parser`
reads TypeScript and TSX for them. `external-config` reports a `prek.toml` without the hook
`eslint-jsdoc`; it does not read `eslint.config.js`.

`external-config` checks each language only in a repository that uses its tool: one with no Python,
or with none of `pyproject.toml`, `ruff.toml` and `.ruff.toml`, needs no configuration of ruff, and
one with no `package.json` at its root needs no hook of ESLint, because the hook runs the ESLint of
the project.

## Configuration

`pyproject.toml`:

```toml
[tool.ruff.lint]
preview = true
explicit-preview-rules = true
select = ["DOC102", "DOC202", "DOC403"]

[tool.ruff.lint.pydocstyle]
convention = "google"
```

`prek.toml`:

```toml
[[repos]]
repo = "local"

[[repos.hooks]]
id = "eslint-jsdoc"
name = "eslint (JSDoc only)"
entry = "npx --no -- eslint --no-warn-ignored"
language = "system"
types_or = ["javascript", "jsx", "ts", "tsx"]
priority = 1
```

With `preview` on, ruff 0.17.1 prints the name of a rule (`docstring-extraneous-parameter`) in
place of its code in its output. A `# noqa` comment still takes the code. A preview rule can change
or be renamed in a release of ruff, so a change of the `rev` of ruff in `prek.toml` is reviewed
with the findings of these three codes.

## Do

```python
def convert(amount: Decimal, rate: Decimal) -> Decimal:
    """Convert an amount at a rate.

    Args:
        amount: The amount, in the currency of the account.
        rate: The units of the target currency for one unit of the account.

    Returns:
        The amount in the target currency.

    """
    return amount * rate
```

```ts
/**
 * Converts an amount at a rate.
 * @param amount - The amount, in the currency of the account.
 * @param rate - The units of the target currency for one unit of the account.
 * @returns The amount in the target currency.
 */
export function convert(amount: number, rate: number): number {
  return amount * rate;
}
```

## Don't

```python
def convert(amount: Decimal) -> None:
    """Convert an amount.

    Args:
        amount: The amount.
        currency: The target currency.

    Returns:
        The converted amount.

    """
    ledger.record(amount)
```

```ts
/**
 * Converts an amount.
 * @param value - The amount.
 * @returns The converted amount.
 */
export function convert(amount: number): void {
  ledger.record(amount);
}
```

## Suppression

A comment of the tool silences a finding, with the reason. In Python it goes on the line of the
finding: `# noqa: DOC102`. In TypeScript and JavaScript ESLint reports the line of the tag, inside
the doc comment, so the comment goes between a
`/* eslint-disable jsdoc/check-param-names -- <reason> */` before it and a
`/* eslint-enable jsdoc/check-param-names */` after the function. A repository that keeps another
convention switches the rule off in the configuration of botica.
