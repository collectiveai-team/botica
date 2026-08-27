---
title: Existing project contracts win — don't bulldoze to match defaults
section: general
scope: general
applies-to: all
status: current
tags: conventions, tooling, migration
---

## Existing project contracts win — don't bulldoze to match defaults

When working in an existing repo, its package manager, lockfile, CI pipeline, configured linter,
and public interfaces are the contracts. Respect them unless the task is an explicit migration.
Imposing a different toolchain mid-feature corrupts lockfiles, breaks CI, and obscures the real diff.

**Prefer:**

```bash
# Repo uses uv — continue to use it for the feature at hand
uv add httpx
uv run pytest
```

**Avoid:**

```bash
# Don't mix two package managers in the same repo
poetry add httpx   # if the repo already uses uv.lock, this breaks reproducibility
pip install httpx  # bypasses the project's dependency management entirely
```

**Direction:** `uv` is the standard package manager for new projects (see `py-package-manager`),
but existing repos migrate only as an explicit, standalone task — never as a side effect of a
feature branch. The same principle applies to formatters, linters, and build backends: don't
swap them inside an unrelated change.
