---
title: Use hatchling as the build backend
section: py
scope: general
applies-to: all
status: current
tags: python, build-backend, hatchling, packaging
---

## Use hatchling as the build backend

`hatchling` (paired with `uv`) is the standard build backend for new packages. Add
`hatch-vcs` to derive the version from git tags for any package consumed as a git dependency
by other repos. `setuptools` is acceptable for a trivial library, but default to hatchling.
Never change the `[build-system]` block as a side effect of a feature — it invalidates the
package metadata and can break CI installs; migrate the backend as its own change.

**Prefer (pyproject.toml):**

```toml
[build-system]
requires = ["hatchling", "hatch-vcs"]
build-backend = "hatchling.build"

[tool.hatch.version]
source = "vcs"            # version from git tags
```

**Avoid:**

```python
# setup.py — legacy; new packages use a pyproject.toml [build-system] block
from setuptools import setup
setup(name="thing", version="0.0.1")
```

Reference: `py-package-manager` (uv), `general-internal-git-deps` (why git-tag versioning matters).
