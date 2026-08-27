---
title: Package dir is named after the import package; keep data dirs out of it
section: general
scope: general
applies-to: all
status: current
tags: layout, repo-structure, conventions
---

## Package dir is named after the import package; keep data dirs out of it

The importable package lives at repo root and is named after what callers import.
Auxiliary dirs — `notebooks/`, `resources/`, `reports/`, `docker-images*/` — sit beside
the package, never inside it. Monorepos split by concern at the top level.

**Prefer:**

```
repo-root/
  mypackage/        # the importable package
  notebooks/        # exploration; outside the package
  resources/        # data assets; outside the package
  docker-images/    # build context; outside the package
  pyproject.toml
```

**Avoid:**

```
repo-root/
  mypackage/
    notebooks/      # pollutes the package namespace
    resources/      # non-code in the installed package
```

**Monorepo shapes:**

- Full-stack repo: `backend/` + `frontend/` split at root.
- Library-plus-experiments repo: package dir alongside `frontend/`, `tests/`, `notebooks/`,
  `resources/`, `docker-images/`, plus numbered experiment dirs (e.g. `01-langfuse/`) at root.
