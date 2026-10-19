## Directive

The cognitive complexity of a Python function is under 15, as complexipy measures it
(`complexipy --max-complexity-allowed 15`).

## Why

Cognitive complexity grows with each level of nesting and with each break in the flow of control
(`if`, `try`, loops, chains of boolean operators), and it follows how hard a person finds a function
to read. Ruff's `C90` counts the branches of a function (cyclomatic complexity), and a function can
pass `C90` while each patch nests one more condition. The two checks run together.

## Tool

[complexipy](https://rohaquinlop.github.io/complexipy/) measures the rule, as a hook of prek. The
hook in `prek.toml` enforces the rule, and `external-config` reports a `prek.toml` without a hook
whose id is `complexipy`.

## Configuration

```toml
[[repos]]
repo = "local"

[[repos.hooks]]
id = "complexipy"
name = "complexipy (cognitive complexity)"
entry = "uvx complexipy --max-complexity-allowed 15"
language = "system"
types = ["python"]
```

In a repository with functions above the limit, `complexipy --snapshot-create` records them, and the
hook then fails on the functions that the snapshot does not list.

## Do

```python
def total(order):
    if not order.items:
        return 0
    return sum(price_of(item) for item in order.items if item.in_stock)
```

## Don't

```python
def total(order):
    result = 0
    if order.items:
        for item in order.items:
            if item.in_stock:
                if item.discount:
                    result += item.price * (1 - item.discount)
                else:
                    result += item.price
    return result
```

## Suppression

complexipy has no comment that silences one function. A file whose functions stay complex on
purpose (a generated parser) is left out with the `exclude` pattern of the hook.
