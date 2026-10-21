## Directive

The cognitive complexity of a function of Python, TypeScript, Go or Rust is at most 15. Each `if`,
loop, `switch` or `match`, `catch` or `except` and ternary adds one plus its nesting; each `else`,
sequence of one boolean operator, labelled `break` or `continue`, `goto` and direct recursive call
adds one. A function, lambda or closure inside another adds to the one that holds it, one level
deeper. Python is counted as [complexipy](https://github.com/rohaquinlop/complexipy) 8.1.0 counts
it.

A function at the top of a module and a method are measured; in Rust, a function of a module, an
`impl` or a `trait` too. The finding covers the function, from its name to the end of its body, and
gives its value and the limit.

## Why

Cognitive complexity grows with each level of nesting and with each break in the flow of control,
and it follows how hard a person finds a function to read. Ruff's `C90` counts the branches of a
function (cyclomatic complexity), and a function can pass `C90` while each patch nests one more
condition. The two checks run together.

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

## Options

- `max` (int, 15 by default): the highest value a function may have; a function above it is
  reported.

## Suppression

A comment on the first line of the function silences it: `# botica: ignore[cognitive-complexity]`
in Python, `// botica: ignore[cognitive-complexity]` in TypeScript, Go and Rust. The comments of
complexipy, `# complexipy: ignore` and `# noqa: complexipy`, do not. A file whose functions stay
complex on purpose, such as a generated parser, goes in the `extend-ignores` of
`[rules.cognitive-complexity]`.
