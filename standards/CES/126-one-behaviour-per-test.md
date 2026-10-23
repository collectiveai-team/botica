---
id: CES-126
title: A test checks one behaviour
---

## Directive

A unit test checks one behaviour of the code under test, and its name says which. A test that
drives several functions of the code to check several outcomes is split into one test per outcome,
with the shared setup in a fixture or a helper. A test of a workflow, such as an end-to-end test,
lives in `tests/integration/` or `tests/e2e/`, where the limit of `eager-test` does not apply.

## Why

- When a test of one behaviour fails, its name says what broke. When a test of five behaviours
  fails, the reader has to find which one it was.
- A test bound to many functions breaks on a change to any of them, and the change that broke it
  may be unrelated to what the test is named for.
- A model that writes the tests of a feature at once tends to put the whole feature in one test.

## Examples

```python
# Do
def test_coupon_halves_the_total():
    cart = Cart(items=[Item(price=10)])
    cart.apply_coupon("HALF")
    assert cart.total() == 5


# Don't: adding, removing, coupons, checkout and receipts in one test
def test_checkout():
    cart = Cart()
    cart.add(1)
    cart.remove(1)
    cart.apply_coupon("A")
    checkout(cart)
    send_receipt(cart)
    assert cart.total() == 0
```
