# no-dict-return-annotation · `BTDM005`

**Standard:** CES-79 (documentation only) · **Family:** data-models · **Languages:** Python

## Directive

A function that crosses a boundary returns a typed structure, never a raw `dict`: a
`@dataclass` for internal boundaries, or a pydantic `BaseModel` where validation or
serialization is needed (a FastAPI endpoint, for one). An error by default.

This rule catches a function (`def` or `async def`) annotated to return `dict` or `dict[...]`.

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

Silence one line with `# botica: ignore[no-dict-return-annotation]` or `# noqa: BTDM005`.
