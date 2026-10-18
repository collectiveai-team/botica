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

- When `core` cannot import `api`, abstractions cannot leak upward and the lower layers stay
  testable in isolation.
- An import-linter contract makes the direction executable: a violating import fails CI before a
  reviewer has to notice it.

## Contract

To enforce the direction, add `import-linter` to the dev group, put this contract in
`pyproject.toml` with the name of your package, and run `uvx --from import-linter lint-imports`:

```toml
[tool.importlinter]
root_package = "myapp"

[[tool.importlinter.contracts]]
name = "Layered architecture (CES-5)"
type = "layers"
layers = [
  "myapp.entrypoints",
  "myapp.api",
  "myapp.database | myapp.impl",
  "myapp.core",
]
```

Sibling modules on one layer are separated with `|` and must be independent of each other.

## Related

- CES-17 (`api-boundary-layout`): the layout that the `api` layer assumes.
- CES-18 (`arch-database-package`): the `database` package that this direction refers to.
