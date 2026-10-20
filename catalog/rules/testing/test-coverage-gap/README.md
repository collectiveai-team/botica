## Directive

A change closes a coverage gap with a test through the interface or by deleting dead code. It does not hide the gap with `# pragma: no cover`, an `omit` of coverage or a lower threshold, and adds no test that runs the lines without asserting a behaviour.

## Why

A percentage is easy to raise without testing anything, and it says little. The uncovered line is where a bug hides, so the gap is the signal: it leads to a real test or to the deletion of code that should not exist.

## Flag

- A `# pragma: no cover` on a branch that a test could reach.
- A file added to `omit` or a pattern to `exclude_lines` of coverage, with no reason given.
- A test that calls code and asserts nothing, or asserts only that no exception was raised.
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

The rule judges a whole change, so only the configuration silences it: `ignore = ["test-coverage-gap"]`, or the paths in its `extend-ignores`.
