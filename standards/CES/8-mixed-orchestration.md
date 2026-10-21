---
id: CES-8
title: Separate orchestration from logic
---

## Directive

Orchestration (the order of steps, control flow, coordinating calls) does not share a function
with business logic and I/O. Keep "what happens in what order" thin and readable, and push "how
each step works" into named units that it calls.

## Why

- A function that fetches, branches, computes, logs and writes all at once has no testable seam:
  the logic cannot be exercised without the I/O, and the flow cannot be followed without reading
  the details.
- Separating the layers makes the orchestration read like a table of contents and lets each step
  be tested in isolation, through its interface (CES-65).

## Smell signals

- Deeply nested `if/for/try` where each level mixes a decision, a calculation and a side effect.
- A single function that both decides what to do and performs all of it.
- Business rules buried inside a request handler or a loop body.

## Do

```python
def place_order(cmd: PlaceOrder) -> OrderId:
    order = build_order(cmd)  # logic (pure)
    validate(order)  # logic (pure)
    return orders.save(order)  # I/O (adapter)
```

The orchestrator coordinates, `build_order` and `validate` hold logic, and `orders` is a port.

## When a function starts mixing flow with detail

Extract the detail into named, separately testable units and leave the orchestration thin.
