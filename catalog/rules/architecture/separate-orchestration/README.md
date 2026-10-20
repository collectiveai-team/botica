## Directive

A function either orders the steps of a task or carries out one of them. The orchestration (the order of the calls and the control flow) does not share a function with the rules of the domain and the I/O; it calls named units that hold them.

## Why

A function that fetches, branches, computes and writes at once has no seam for a test: the logic cannot run without the I/O, and the flow cannot be read without the details. With the orchestration apart, it reads as a list of steps and each step is tested alone.

## Flag

- A function where one level of nesting mixes a decision, a calculation and a side effect.
- A request handler or a loop body that holds a rule of the domain inline, next to the I/O.
- A function that decides what to do and also performs every step of it.

## Do not flag

- A short function that calls one adapter and returns its result.
- A pure function that computes one thing, however many branches it has.
- An entry point that parses its arguments and calls one orchestrator.

## Confirm

Before reporting, name the steps that the function mixes and the unit each one would move to.

## Do

```python
def place_order(cmd: PlaceOrder) -> OrderId:
    order = build_order(cmd)
    validate(order)
    return orders.save(order)
```

## Don't

```python
def place_order(cmd: PlaceOrder) -> OrderId:
    rows = db.execute("select price from items where id in :ids", ids=cmd.item_ids)
    total = 0
    for row in rows:
        if row.price > 100 and cmd.coupon:
            total += row.price * 0.9
        else:
            total += row.price
    if total > cmd.limit:
        raise LimitExceeded(total)
    return db.insert("orders", total=total)
```

## Suppression

A file that stays mixed on purpose takes `botica: ignore-file[separate-orchestration] -- <reason>` in a comment. A folder goes in the `extend-ignores` of the rule.
