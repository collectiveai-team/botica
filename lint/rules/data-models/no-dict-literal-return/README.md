## Directive

A function that crosses a boundary returns a typed structure, never a raw `dict`: a
`@dataclass` for internal boundaries, or a pydantic `BaseModel` where validation or
serialization is needed (a FastAPI endpoint, for one). An error by default.

This rule catches a function that returns a dict literal: `return { ... }`.

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
def payload():
    return {"ok": True}
```

## Suppression

A comment on the `return` line silences it.

```python
def payload():
    return {"ok": True}  # botica: ignore[no-dict-literal-return]
    return {"ok": True}  # noqa: BTDM004 -- reason
```
