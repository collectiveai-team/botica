# no-dict-alias · `BTDM002`

**Standard:** CES-79 (documentation only) · **Family:** data-models · **Languages:** Python

## Directive

A function that crosses a boundary returns a typed structure, never a raw `dict`: a
`@dataclass` for internal boundaries, or a pydantic `BaseModel` where validation or
serialization is needed (a FastAPI endpoint, for one). An error by default.

This rule catches a type alias that hides a raw dict behind a name: `X = dict`,
`X = dict[...]`, `X: TypeAlias = dict[...]` and `type X = dict[...]`.

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
Payload = dict
Counts = dict[str, int]
```

## Suppression

Silence one line with `# botica: ignore[no-dict-alias]` or `# noqa: BTDM002`. The rule is written in ast-grep's
YAML (`python.yml`), so `# ast-grep-ignore: no-dict-alias` also works while `compat-ast-grep-ignore` is on.
