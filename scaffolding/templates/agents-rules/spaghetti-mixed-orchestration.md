# CES-8 · separate orchestration from logic

**Code:** `CES-8` &nbsp;·&nbsp; **Slug:** `spaghetti-mixed-orchestration` &nbsp;·&nbsp; **Tier:**
judgment &nbsp;·&nbsp; **Tracker:** [#8](https://github.com/collectiveai-team/botica/issues/8)

## Directive

Don't interleave **orchestration** (the order of steps, control flow, coordinating calls) with
**business logic** and **I/O** in one tangled function. Keep the "what happens in what order" thin
and readable; push the "how each step works" into named units it calls.

## Why

- A function that fetches, branches, computes, logs, and writes all at once has no testable seam:
  you can't exercise the logic without the I/O, and you can't follow the flow without reading the
  details. That is the spaghetti.
- Separating the layers makes the orchestration read like a table of contents and lets each step
  be tested in isolation (through its interface — CES-65).

## Smell signals

- Deeply nested `if/for/try` where each level mixes a decision, a calculation, and a side effect.
- A single function that both decides *what* to do and performs *all* of it.
- Business rules buried inside a request handler or a loop body.

## Do

```python
def place_order(cmd: PlaceOrder) -> OrderId:
    order = build_order(cmd)          # logic (pure)
    validate(order)                   # logic (pure)
    return orders.save(order)         # I/O (adapter)
```

The orchestrator coordinates; `build_order`/`validate` hold logic; `orders` is a port.

## Judgment

No linter catches this — it is a structural judgment. When a function starts mixing flow with
detail, extract the detail into named, separately-testable units and leave the orchestration thin.
