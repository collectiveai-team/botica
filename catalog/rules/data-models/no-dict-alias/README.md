## Directive

A boundary function returns a typed structure, never a raw `dict`: a `@dataclass` for internal
boundaries, or a pydantic `BaseModel` where validation or serialization is needed (a FastAPI
endpoint, for one). An error by default.

A boundary function is a function of a module or a method of a class, whose name and whose class
name do not start with `_`, and that is not defined inside another function. The rule skips private
and nested functions.

This rule catches a type alias that hides a raw dict behind a name: `X = dict`,
`X = dict[...]`, `X: TypeAlias = dict[...]` and `type X = dict[...]`. It looks at the aliases of a module whose
name does not start with `_`; an assignment inside a function is a local variable.

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
Payload = dict
Counts = dict[str, int]
```

## Suppression

A comment on the line of the alias silences it.

```python
Payload = dict  # botica: ignore[no-dict-alias]
Payload = dict  # noqa: BTDM002 -- reason
```
