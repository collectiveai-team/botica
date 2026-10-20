---
id: CES-79
title: No raw dict at a boundary
references: ["typed-boundaries"]
---

## Directive

A boundary function does not return a bare `dict` and a public type alias does not point at one.
Use a `@dataclass` for an internal boundary and a `pydantic` `BaseModel` where validation is
needed. Returning a `{...}` literal or the result of `dict(...)` from a boundary function is the
same mistake.

A boundary function is one that another module can call: a function of a module or a method of a
class, whose name and whose class name do not start with `_`, and that is not defined inside another
function. An endpoint, a command of a command line program and the public functions of a layer are
boundary functions. A private function (`_name`) and a nested function are internal, and may use a
`dict`.

## Why

A raw `dict` carries no field names or types for callers and the type checker. A named type
declares them.
