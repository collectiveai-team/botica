---
id: CES-58
title: One modern lint stack
---

## Directive

Python code uses one toolchain: ruff (format and lint), pyrefly (types) and botica (the rules that
can be checked deterministically), all run through prek. The legacy stack is not reintroduced:
`black`, `isort`, `flake8` (and its plugins), `pylint`, `autopep8`, `yapf` and standalone
`pyupgrade`.

## Why

- ruff covers black, isort, flake8, pyupgrade and most of pylint in one fast tool with one config.
  Stacking the old tools on top means overlapping, conflicting passes and a config spread across
  five files.
- One toolchain is one source of truth for what clean code looks like, a faster CI and no fights
  between formatters.

## Replacements

| Legacy | Use |
|---|---|
| `black`, `yapf`, `autopep8` | `ruff format` |
| `isort` | `ruff` (import sorting, `I`) |
| `flake8` + plugins | `ruff` lint rules |
| `pyupgrade` | `ruff` (`UP`) |
| `pylint` | `ruff` + `pyrefly` |
| mypy (as the only checker) | `pyrefly` |

## When a legacy tool appears

A legacy tool in `pyproject.toml` or `prek.toml` is removed, and its intent is expressed through
this stack.
