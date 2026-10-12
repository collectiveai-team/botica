# CES-5 · layered import direction

**Code:** `CES-5` &nbsp;·&nbsp; **Slug:** `import-linter` &nbsp;·&nbsp; **Enforced by:** AGENTS
directive now; a **commented** `[tool.importlinter]` skeleton in `pyproject.toml` (uncomment once
the package has layers) &nbsp;·&nbsp; **Tracker:**
[#5](https://github.com/collectiveai-team/botica/issues/5)

## Directive

Imports flow in one direction through the house layers:

```
entrypoints  →  api  →  database / impl  →  core
```

A higher layer may import lower ones; a lower layer must **never** import a higher one (`core`
imports nothing app-specific; `database`/`impl` never import `api`; `api` never imports
`entrypoints`). This keeps the dependency graph acyclic and the core reusable.

## Why

- Layer direction is what stops a codebase from congealing into a ball of mud: when `core` can't
  import `api`, the abstractions can't leak upward and the lower layers stay testable in
  isolation.
- Encoding it as an import-linter contract makes the rule *executable* — a violating import
  fails CI instead of waiting for a reviewer to notice.

## Enforcement status

This slice ships the **directive + a commented contract skeleton** in `pyproject.toml`. Active
enforcement is opt-in: uncomment `[tool.importlinter]`, replace `replace-me` with your package,
add `import-linter` to the dev group, and run `uvx --from import-linter lint-imports`. Keeping it
commented avoids failing a repo that doesn't yet have the layers carved out.

## The skeleton

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

(Sibling modules on one layer are separated with `|` — they must be independent of each other.)

## Related

- **CES-17** (`api-boundary-layout`) — the placement layout the api layer assumes.
- **CES-18** (`arch-database-package`) — the `database` package this direction references.
