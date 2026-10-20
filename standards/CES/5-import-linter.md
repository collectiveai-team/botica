---
id: CES-5
title: Layered import direction
---

## Directive

Imports flow in one direction through these layers:

```
entrypoints  →  api  →  database / impl  →  core
```

A higher layer may import lower ones. A lower layer never imports a higher one: `core` imports
nothing app-specific, `database` and `impl` never import `api`, and `api` never imports
`entrypoints`. The dependency graph stays acyclic and `core` stays reusable.

## Why

When `core` cannot import `api`, abstractions cannot leak upward and the lower layers stay testable
without the upper ones. A rule makes a violating import fail the run before a reviewer has to
notice it.
