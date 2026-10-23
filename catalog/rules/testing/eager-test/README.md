## Directive

A test calls at most `max` distinct functions of the code under test, 5 by default. The rule counts
the names of the functions and methods that the body of each test calls, each name once, and
reports a test above the limit, from its first line to its last. A warning by default.

| Language | A test is | Not counted |
|---|---|---|
| Python | a `test*` function at the top of a module, or a `test*` method of a `Test*` class | `assert*`, a name that starts with `_` or a capital letter (a class), builtins such as `len` and `sorted`, `pytest.raises`, `approx`, `patch` |
| TypeScript, TSX, JavaScript | the callback of `it(…)` or `test(…)` | a chain that starts with `expect`, `assert`, `vi`, `jest`, `console`, `JSON`, `Object`, `Array`, `Math` or `Promise`; `describe`, `it`, `test` and the hooks |
| Go | a `Test*` function | a method of the `*testing.T` parameter, `assert`, `require`, `fmt`, `errors`, `strings`, `strconv`; builtins such as `len` and `make` |
| Rust | a function with `#[test]` or `#[<crate>::test]` | macros (`assert_eq!`), `new`, `default`, `from`, `unwrap`, `expect`, `clone`, `iter`, `collect` and the other conversions |

## Why

A test that drives many functions of the code at once checks several behaviours: when it fails,
its name does not say which one broke, and a change to any of them breaks it. The smell is the
"eager test" of the test smell catalog. A count of names is a proxy, so the rule is a warning and
its limit is an option.

## Exceptions

A test of a workflow, such as an end-to-end test, calls many functions on purpose. The rule leaves
out `tests/integration/` and `tests/e2e/` at the root of the project by default; another folder of
such tests goes in `extend-ignores` of `[rules.eager-test]` (`"**/tests/integration/**"` in a
repository with several packages), and `ignores = []` puts the default folders back in.

A factory of the setup (`make_user`, `create_order`, `build_cart`) counts like any other function,
and so does each step of a builder or a fluent interface (`order.with_item(…).with_coupon(…)`). A
test whose setup needs many of them moves the setup to a fixture.

## Do

```python
def test_coupon_lowers_the_total():
    cart = Cart(items=[Item(price=10)])
    cart.apply_coupon("HALF")
    assert cart.total() == 5
```

## Don't

```python
def test_checkout():
    cart = Cart()
    cart.add(1)
    cart.remove(1)
    cart.apply_coupon("A")
    checkout(cart)
    send_receipt(cart)
    assert cart.total() == 0
```

## Options

- `max` (int, at least 1, default 5): the number of distinct functions a test may call.

## Suppression

`# botica: ignore[eager-test]` on the first line of the test in Python,
`// botica: ignore[eager-test]` in the other languages, with the reason.
