---
id: CES-17
title: API boundary layout
---

## Directive

All inbound HTTP lives in one `api` package with a fixed, versioned shape. Request and response
schemas live under `api/<version>/schemas/{requests,responses}/`, and routers and endpoints under
`api/<version>/routers/`. The `api` package is the only inbound boundary: `core` and `database`
never import it.

## Why

- A predictable layout lets tooling reason about code by placement instead of guesswork: CES-4
  (`api-schemas-extra-forbid`) fires only on models under `api/**/schemas/...`, so it stays silent
  for internal models without inferring intent.
- Versioned subpackages (`v1`, `v2`) make breaking API changes additive: a new version is a new
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

## Enforcement

The rules of botica that depend on the layout read it from the path of each file. The direction,
that nothing outside `api` imports `api`, is an import-linter contract (CES-5).

## Related

- CES-4 (`api-schemas-extra-forbid`): the schema rule scoped to this layout.
- CES-5 (`import-linter`): the layer direction that protects this boundary.
