---
id: CES-132
title: A port has two adapters, and a wrapper adds a step
---

## Directive

- A port has two adapters. The in-memory adapter of the tests (CES-64) is the second one, so a
  port over a database, a network service or a third-party API has its two adapters once its fake
  is written. When a port has one adapter and the dependency behind it is pure computation or
  in-memory state, its callers use that class, and the port is deleted.
- A class or a function adds a step to the calls it receives: a validation, a mapping, a retry, a
  default or a decision. When it only forwards each call to another object, its callers call that
  object, and the class or the function is deleted.

## Why

The skill `codebase-design` of the recipe `base` states both checks. On ports: "One adapter means
a hypothetical seam. Two adapters means a real one. Don't introduce a seam unless something
actually varies across it." On wrappers, the deletion test: "Imagine deleting the module. If
complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was
earning its keep."

A skill holds no rule (ADR 0008). The rules `port-has-two-adapters` and `no-pass-through` put the
two checks in the rubric of abide, which judges each edit.

## Examples

A port with one adapter over a calculation:

```python
class PricingStrategy(Protocol):
    def price(self, subtotal: int, quantity: int) -> int: ...


class StandardPricing:
    def price(self, subtotal: int, quantity: int) -> int:
        return subtotal * 95 // 100 if quantity >= 10 else subtotal
```

The callers call `StandardPricing` and the `Protocol` is deleted. A port `Clock` with
`SystemClock` and the `FixedClock` of the tests has two adapters.

A wrapper that only forwards:

```python
class OrderService:
    def get(self, order_id: OrderId) -> Order:
        return self._orders.get(order_id)
```

The callers call `orders.get(order_id)`.
