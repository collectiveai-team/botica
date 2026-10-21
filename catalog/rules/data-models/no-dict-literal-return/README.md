## Directive

A boundary function returns a typed structure, never a raw `dict`: a `@dataclass` for internal
boundaries, or a pydantic `BaseModel` where validation or serialization is needed (a FastAPI
endpoint, for one). An error by default.

A boundary function is a function of a module or a method of a class, whose name and whose class
name do not start with `_`, and that is not defined inside another function. The rule skips private
and nested functions.

This rule catches a function that returns a dict literal: `return { ... }`.

A lookup table, whose keys are data and not field names, is declared with an alias whose name ends
in `Table` (`StatusTable = dict[str, str]`); the rule accepts it.

## Why

- The type checker cannot check the keys of a `dict[str, Any]`, so a renamed key breaks its
  callers at run time.
- A `@dataclass` or `BaseModel` declares the fields, and the type checker checks them.

## Do

```python
from dataclasses import dataclass


@dataclass
class Point:
    x: int
    y: int


def origin() -> Point:
    return Point(x=0, y=0)
```

## Don't

```python
def payload():
    return {"ok": True}
```

## Suppression

A comment on the `return` line silences it.

```python
def payload():
    return {"ok": True}  # botica: ignore[no-dict-literal-return]
    return {"ok": True}  # noqa: BTDM003 -- reason
```
