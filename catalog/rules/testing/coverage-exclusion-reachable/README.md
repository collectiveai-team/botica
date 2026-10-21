## Directive

A change leaves out of coverage only code that no test can reach. It does not hide a gap with `# pragma: no cover`, an `omit` or `exclude_lines` of coverage, or a lower threshold.

## Why

A percentage is easy to raise without testing anything, and it says little. The uncovered line is where a bug hides, so the gap is the signal: it leads to a real test or to the deletion of code that should not exist.

## Flag

- A `# pragma: no cover` on a branch that a test could reach.
- A file added to `omit` or a pattern to `exclude_lines` of coverage, with no reason given.
- A lower `fail_under`.

## Do not flag

- `# pragma: no cover` on code that cannot run under test, such as an `if __name__ == "__main__":` block or a branch for another platform.
- Generated code left out of coverage.
- A change that deletes dead code and the coverage it had.

## Confirm

Before reporting, check whether a test through the public interface could reach the excluded code.

## Do

```python
def test_an_empty_order_costs_nothing():
    assert total(Order(items=[])) == 0
```

## Don't

```python
def total(order):
    if not order.items:  # pragma: no cover
        return 0
    return sum(item.price for item in order.items)
```

## Suppression

The rule judges a whole change, so only the configuration silences it: `ignore = ["coverage-exclusion-reachable"]`, or the paths in its `extend-ignores`.
