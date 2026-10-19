---
id: CES-79
title: No raw dict at a boundary
references: ["typed-boundaries"]
---

## Directive

A function does not return a bare `dict` and a type alias does not point at one. Use a
`@dataclass` for an internal boundary and a `pydantic` `BaseModel` where validation is needed.
Returning a `{...}` literal or the result of `dict(...)` from a boundary function is the same
mistake.

## Why

A raw `dict` carries no field names or types for callers and the type checker. A named type
declares them.

## Exceptions

A lookup table is a `dict`: its keys are data (a slug, an id, a title), not the names of fields.
Declare it with an alias whose name ends in `Table` and annotate with the alias:

```python
StatusTable = dict[str, str]


def statuses(root: Path) -> StatusTable:
    return {rule.id: rule.status for rule in rules_of(root)}
```

The rules of this standard accept such an alias, and a `return` of a `dict` in a function whose
return type is one. A record (fields known in advance) stays a `@dataclass` or a `BaseModel`.

When another `dict` is correct, as at a JSON serialization boundary, keep the annotation and suppress the
finding at that site (`# botica: ignore[<slug>]`), so the exception is visible.
