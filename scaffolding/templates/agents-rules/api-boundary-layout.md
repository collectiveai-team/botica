# CES-17 · API boundary layout

**Code:** `CES-17` &nbsp;·&nbsp; **Slug:** `api-boundary-layout` &nbsp;·&nbsp; **Enforced by:**
AGENTS directive + placement-scoped ast-grep rules that depend on this layout (e.g. CES-4); a
**commented** import-linter forbidden-contract skeleton in `pyproject.toml` &nbsp;·&nbsp;
**Tracker:** [#17](https://github.com/collectiveai-team/botica/issues/17)

## Directive

All inbound HTTP lives in one `api` package with a fixed, versioned shape. Request/response
schemas live under `api/<version>/schemas/{requests,responses}/`; routers/endpoints under
`api/<version>/routers/`. The `api` package is the **only** inbound boundary — `core` and
`database` never import it.

## Why

- A predictable layout lets tooling reason about code by **placement** instead of guesswork:
  CES-4 (`api-schemas-extra-forbid`) fires only on models under `api/**/schemas/...`, so it can
  stay silent for internal models without trying to infer intent.
- Versioned subpackages (`v1`, `v2`) make breaking API changes additive — a new version is a new
  directory, not an edit that ripples through shared modules.

## Layout

```text
myapp/
  api/
    v1/
      routers/
        users.py
      schemas/
        requests/
          create_user.py     # BaseModel + ConfigDict(extra="forbid")  (CES-4)
        responses/
          user.py
  core/        # never imports api
  database/    # never imports api
```

## Enforcement status

The layout is documented here and assumed by the placement-scoped ast-grep rules. The
**direction** (nothing outside `api` may import `api`) ships as a commented
`[tool.importlinter]` forbidden-contract skeleton in `pyproject.toml` — uncomment it once the
package has these layers. Active import-linter enforcement is otherwise out of scope.

## Related

- **CES-4** (`api-schemas-extra-forbid`) — the schema rule scoped to this layout.
- **CES-5** (`import-linter`) — the layer direction that protects this boundary.
