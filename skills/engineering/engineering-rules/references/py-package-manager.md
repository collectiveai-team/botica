---
title: Use uv to manage dependencies; commit the lockfile
section: py
scope: general
applies-to: all
status: current
tags: python, package-manager, uv, dependencies
---

## Use uv to manage dependencies; commit the lockfile

`uv` is the standard package manager for Python projects. Manage dependencies with `uv`
and always commit `uv.lock` so installs are reproducible across machines and CI. Repos
still on Poetry or bare `pip`/`setup.py` should migrate to `uv`; do it as a deliberate,
standalone change and never run two managers against the same project at once.

**Prefer:**

```bash
uv add httpx && git add uv.lock     # add a dep and lock it
uv sync                             # reproducible install from the lockfile
uv run pytest                       # run inside the project environment
```

**Avoid:**

```bash
pip install httpx                   # bypasses the project's dependency management
poetry add httpx                    # keep new work on uv; migrate the repo instead
# committing code without the updated uv.lock
```

Reference: see `py-build-backend` (hatchling + uv) and `general-respect-local-repo` for how to
sequence a migration without breaking a working repo mid-change.
