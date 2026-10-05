# CES-79 · no raw dicts at boundaries

**Code:** `CES-79` &nbsp;·&nbsp; **Slugs:** `no-dict-return-annotation`, `no-dict-call-return`,
`no-dict-literal-return`, `no-dict-alias` &nbsp;·&nbsp; **Enforced by:** ast-grep (`error`)
&nbsp;·&nbsp; **Tracker:** [#79](https://github.com/collectiveai-team/botica/issues/79)

## Directive

Functions that cross a boundary must return a **typed structure**, never a raw `dict`. Use a
`@dataclass` for internal boundaries, or a pydantic `BaseModel` where validation/serialization
is needed (e.g. FastAPI request/response models). Raw dicts erase field names and types at the
exact point another module starts depending on them.

## Why

- A `dict[str, Any]` boundary is an untyped contract: callers guess keys, refactors silently
  break, and the type checker can't help.
- A `@dataclass`/`BaseModel` makes the shape explicit, greppable, and checkable, and gives you
  one obvious place to add validation later.

## What the rule catches

This CES ships as four ast-grep patterns so the raw-dict boundary can't slip through any form:

| Slug | Catches |
|---|---|
| `no-dict-return-annotation` | `def f(...) -> dict:` / `-> dict[...]` (and `async def`) |
| `no-dict-call-return` | `return dict(...)` |
| `no-dict-literal-return` | `return { ... }` inside a function |
| `no-dict-alias` | `X = dict` / `type X = dict[...]` aliases that hide a raw-dict boundary |

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

See `.agents/snippets/no-dict-boundary.py` for the canonical drop-in / comparison template.

## Don't

```python
def origin() -> dict:          # no-dict-return-annotation
    return {"x": 0, "y": 0}    # no-dict-literal-return
```

## Suppressing (rare, must be visible)

A raw `dict` is sometimes genuinely correct (e.g. a JSON-serialization boundary). Keep the
`dict` and add an explicit, visible suppression at the call site so the exception is auditable:

```python
def payload() -> dict:  # ast-grep-ignore: no-dict-return-annotation
    return {"ok": True}
```

The slug is the stable suppression key; the `CES-79` code in the violation message is for
humans and never affects tooling.
