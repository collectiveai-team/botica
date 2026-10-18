---
title: CES-58 · one modern lint stack
section: py
scope: general
applies-to: python
status: current
tags: python, black, isort, pylint, flake8, mypy, formatting, linting, migration
---

# CES-58 · one modern lint stack

**Code:** `CES-58` &nbsp;·&nbsp; **Slug:** `py-legacy-lint-stack` &nbsp;·&nbsp; **Tier:** judgment
&nbsp;·&nbsp; **Tracker:** [#58](https://github.com/collectiveai-team/botica/issues/58)

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
