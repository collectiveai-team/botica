---
id: CES-79
title: No raw dict at a boundary
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

When a `dict` is correct, as at a JSON serialization boundary, keep the annotation and suppress the
finding at that site (`# botica: ignore[<slug>]`), so the exception is visible.
