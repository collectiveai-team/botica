## Directive

A class or a function that only forwards each call to another object is deleted, and its callers call that object.

## Why

The skill `codebase-design` of the recipe `base` calls this check the deletion test: "Imagine deleting the module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep."

## Flag

- A method that returns the call of the same method of the object it holds, with the same arguments.
- A module of functions that each call one function of another module with the same arguments.
- A wrapper class whose methods all forward to one client.

## Do not flag

- A port, and an adapter that translates a library to a port of the project.
- A function that orders the steps of a use case, even when it calls one adapter.
- A unit that changes the call or its result, for example with a default or a validation.
- A dependency that a framework asks for, such as a FastAPI dependency.

## Confirm

Before reporting, name the unit, the object it forwards to, and the code its callers would hold after the deletion.

## Do

```python
order = orders.get(order_id)
```

## Don't

```python
class OrderService:
    def __init__(self, orders: Orders) -> None:
        self._orders = orders

    def get(self, order_id: OrderId) -> Order:
        return self._orders.get(order_id)
```

## Suppression

A pass-through kept on purpose, such as a seam for a planned change, takes `botica: ignore-file[no-pass-through] -- <reason>` in a comment of its file.
