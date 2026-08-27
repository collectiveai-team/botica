---
title: Standard lint stack — ruff + ruff-format + pyrefly + ast-grep, run through prek, line 100
section: py
scope: general
applies-to: all
status: current
tags: python, ruff, pyrefly, ast-grep, prek, formatting, linting, toolchain
---

## Standard lint stack — ruff + ruff-format + pyrefly + ast-grep, run through prek, line 100

The house Python toolchain is **ruff** for lint + format (100-char lines), **pyrefly** for type
checking, and **ast-grep** for structural rules — all wired through **prek** (`prek.toml`), not
`pre-commit`. New projects use this; repos still on black/isort/pylint or mypy-as-the-only-checker
migrate to it (see `py-legacy-lint-stack`, CES-58). Don't run two formatters at once.

**Prefer (pyproject.toml):**

```toml
[tool.ruff]
line-length = 100

[tool.ruff.format]
quote-style = "double"

[tool.ruff.lint]
# Excerpt of the house selection (correctness + security + modernization + docstrings).
# The full canonical list is the scaffolder's pyproject template — copy it from there rather
# than retyping this one; it also carries the D2xx/D4xx docstring rules and PLE/PLW/TC/PGH.
select = ["E", "F", "I", "C4", "C90", "B", "S", "G", "SIM", "RET", "PTH", "LOG", "TID", "UP", "RUF", "PERF"]
task-tags = ["TODO", "FIXME", "XXX", "MARK"]

[tool.ruff.lint.pydocstyle]
convention = "google"

[tool.ruff.lint.mccabe]
max-complexity = 10

# Absolute package imports only — modules stay unambiguous and movable.
[tool.ruff.lint.flake8-tidy-imports]
ban-relative-imports = "all"

[tool.ruff.lint.per-file-ignores]
"tests/**" = ["S101", "INP001"]

[tool.pyrefly]
project-excludes = ["**/node_modules", "**/__pycache__", "**/.venv/**/*"]
use-ignore-files = false
disable-project-excludes-heuristics = true
```

**Prefer (prek.toml — prek is the hook runner; it reads pre-commit-format hook repos):**

```toml
# MARK: ruff-format
[[repos]]
repo = "https://github.com/astral-sh/ruff-pre-commit"
rev = "v0.15.22"

[[repos.hooks]]
id = "ruff-format"
priority = 0

# MARK: ruff-check
[[repos.hooks]]
id = "ruff-check"
args = ["--fix", "--exit-non-zero-on-fix"]
types_or = ["python", "pyi"]
require_serial = true
priority = 1

# MARK: pyrefly
[[repos]]
repo = "local"

[[repos.hooks]]
id = "pyrefly"
name = "pyrefly"
entry = "uvx pyrefly check --config pyproject.toml"
language = "system"
types = ["python"]
pass_filenames = false
priority = 2

# MARK: ast-grep
[[repos.hooks]]
id = "ast-grep"
name = "ast-grep"
entry = "uvx --from ast-grep-cli ast-grep scan"
language = "system"
types = ["python"]
pass_filenames = false
priority = 2
```

**Avoid:**

```toml
# Don't introduce black or isort alongside ruff — they conflict
[tool.black]
line-length = 88

# Don't add mypy as the type checker — pyrefly is the house checker
[tool.mypy]
python_version = "3.13"
```

```yaml
# Don't add a .pre-commit-config.yaml — hooks are declared in prek.toml
```

This replaces `black` + `isort` + `pylint` + `mypy`. Repos still on that stack migrate as a
deliberate change (see `py-legacy-lint-stack`); don't add ruff alongside them. Structural rules
live in `ast-grep/rules/*.yml` with `sgconfig.yml` pointing at that directory.
