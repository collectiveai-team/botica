## Directive

A port has two adapters: the one of production and, usually, the in-memory adapter of the tests. When a port has one adapter and the dependency behind it is pure computation or in-memory state, the callers use that class, and the port is deleted.

## Why

The skill `codebase-design` of the recipe `base` says: "One adapter means a hypothetical seam. Two adapters means a real one. Don't introduce a seam unless something actually varies across it." Its `DEEPENING.md` counts the adapter of the tests: "at least two adapters are justified (typically production + test)". `test-in-memory-adapters` asks for one in-memory adapter per port, so a port over I/O gets its second adapter when its fake is written.

## Flag

- A `Protocol` or an ABC with one class that implements it, over a calculation, and no fake in the tests.
- An interface of TypeScript that one class implements, injected where the class would do.
- A port that no class implements.

## Do not flag

- A port with an in-memory adapter in the tests, such as `Clock` with `SystemClock` and `FixedClock`.
- A port over a database, a network service, a third-party API, the clock or the filesystem, even
  without its fake: the missing fake is a finding of `test-in-memory-adapters`.
- A `Protocol` that only types the argument of a function and that callers never inject.

## Confirm

Before reporting, name the port, its only adapter, and the dependency behind it.

## Do

```python
def price(order: Order, rules: PricingRules) -> Money:
    return rules.apply(order.subtotal)
```

## Don't

```python
class PricingStrategy(Protocol):
    def price(self, order: Order) -> Money: ...


class StandardPricing:
    def price(self, order: Order) -> Money:
        return order.subtotal
```

## Suppression

A port kept for an adapter that is on its way takes `botica: ignore-file[port-has-two-adapters] -- <reason>` in a comment of its file.
