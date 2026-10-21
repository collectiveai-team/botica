## Directive

A change closes a coverage gap with a test that asserts a behaviour, or by deleting dead code. It adds no test that runs the lines without asserting what they do.

## Why

A percentage is easy to raise without testing anything, and it says little. A test that calls code and asserts nothing marks its lines as covered, and a bug on them stays unseen: the gap is still there, and the report no longer shows it.

## Flag

- A test that calls code and asserts nothing.
- A test whose only assertion is that no exception was raised, or that the result is not `None`.

## Do not flag

- A test that asserts on what the code returns, raises, writes or sends.
- A test of a function whose behaviour is to raise or to return nothing, that asserts on that.
- A change that does not add or change tests.

## Confirm

Before reporting, check that no assertion of the test, its fixtures or a helper it calls reads the result of the code under test.

## Do

```python
def test_an_empty_order_costs_nothing():
    assert total(Order(items=[])) == 0
```

## Don't

```python
def test_total_runs():
    total(Order(items=[]))
```

## Suppression

The rule judges a whole change, so only the configuration silences it: `ignore = ["test-coverage-gap"]`, or the paths in its `extend-ignores`.
