---
title: Pin internal git dependencies to a tag, never a branch
section: general
scope: general
applies-to: all
status: current
tags: dependencies, git, versioning, internal-packages
---

## Pin internal git dependencies to a tag, never a branch

Internal packages consumed across repos are installed as git dependencies.
Always pin to an immutable tag. Pinning to a branch (`@main`, `@develop`) makes the
lockfile non-reproducible — the same `uv lock` or `pip install` can resolve different
code on different days.

**Prefer:**

```toml
# pyproject.toml — exact tag
"mylib @ git+https://github.com/your-org/mylib.git@v0.0.2"
"otherlib @ git+https://github.com/your-org/otherlib.git@v1.3.0"
```

**Avoid:**

```toml
# Non-reproducible — resolves to whatever HEAD is at install time
"mylib @ git+https://github.com/your-org/mylib.git@main"
```

When bumping a dep, update the tag and re-lock (`uv lock`) in the same commit so the
lockfile stays in sync.
