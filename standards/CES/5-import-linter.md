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
- A rule makes the direction executable: a violating import fails the run before a reviewer has to
  notice it.

## Rule

The botica rule `import-layers` checks the direction. List the layers of the package from the top
one down in the configuration of botica, with `|` between the modules of one layer that are
independent of each other:

```toml
[rules.import-layers]
layers = [
  "myapp.entrypoints",
  "myapp.api",
  "myapp.database | myapp.impl",
  "myapp.core",
]
```

It reports an import of a layer above, and an import that reaches one through modules that are in
no layer, with the chain of modules. `botica explain import-layers` describes it.

## Related

- CES-17 (`api-boundary-layout`): the layout that the `api` layer assumes.
- CES-18 (`arch-database-package`): the `database` package that this direction refers to.
