## Directive

A Python comment `# pragma: no cover`, the marker that coverage.py leaves out of its report, is
reported, in any case and spacing that coverage.py reads (`#pragma:no cover`, `# PRAGMA: NO COVER`).
A warning by default. With `botica lint --diff`, only a pragma on a line that the change adds is
reported.

## Why

A pragma raises the percentage of coverage and tests nothing: the line it marks has no test, and
a bug on it stays unseen. The gap is a missing test, dead code, or a seam to refactor, and each has
a fix that is not the pragma.

## Do

```python
def test_an_empty_order_costs_nothing():
    assert total(Order(items=[])) == 0
```

```python
if __name__ == "__main__":  # pragma: no cover  # noqa: BTTEST004 -- a script
    main()
```

## Don't

```python
def total(order):
    if not order.items:  # pragma: no cover
        return 0
    return sum(item.price for item in order.items)
```

## Suppression

`# botica: ignore[no-coverage-pragma] -- <reason>` or `# noqa: BTTEST004 -- <reason>` on the line
of the pragma, for code that cannot run under test: the block of `if __name__ == "__main__":`, a
branch for another platform, or generated code. A file of generated code goes in the
`extend-ignores` of `[rules.no-coverage-pragma]`.
