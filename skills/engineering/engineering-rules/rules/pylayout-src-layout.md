---
title: Use src/ only if the repo already does; never double-nest the package
section: pylayout
scope: general
applies-to: all
status: current
tags: layout, src-layout, package-structure
---

## Use src/ only if the repo already does; never double-nest the package

Most Python repos use flat package-at-root layout. Don't introduce a `src/` layer as a
side effect of adding files. If a repo already has `src/`, continue using it. Never
double-nest the package directory inside itself — this produces broken imports that are
invisible until install time.

**Prefer (flat — the common shape):**

```
repo-root/
  mypackage/         # importable as `import mypackage`
    adapters/
    meta/
  pyproject.toml
```

**Prefer (src/ — only if the repo already uses it):**

```
repo-root/
  src/
    mypackage/        # importable as `import mypackage`
  pyproject.toml
```

**Avoid (double-nested — broken import path):**

```
repo-root/
  src/
    mypackage/
      mypackage/  # double-nested — `import mypackage` resolves to wrong path
```

When in doubt, match the existing repo shape. A `src/` migration is a standalone task.
