## Directive

A boundary function returns a typed structure, never a raw `dict`: a `@dataclass` for internal
boundaries, or a pydantic `BaseModel` where validation or serialization is needed (a FastAPI
endpoint, for one). An error by default.

A boundary function is a function of a module or a method of a class, whose name and whose class
name do not start with `_`, and that is not defined inside another function. The rule skips private
and nested functions.

This rule catches a function (`def` or `async def`) annotated to return `dict` or `dict[...]`.

A lookup table, whose keys are data and not field names, is declared with an alias whose name ends
in `Table` (`StatusTable = dict[str, str]`); the rule accepts it (CES-79, Exceptions).

## Why

- A `dict[str, Any]` boundary is an untyped contract: callers guess keys, a refactor breaks them
  silently and the type checker cannot help.
- A `@dataclass` or `BaseModel` makes the shape explicit, searchable and checkable, and gives
  validation one obvious place to live.

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
def payload() -> dict:
    return load()
```

## Suppression

A comment on the `def` line silences it.

```python
def payload() -> dict:  # botica: ignore[no-dict-return-annotation]
def payload() -> dict:  # noqa: BTDM004 -- reason
```
