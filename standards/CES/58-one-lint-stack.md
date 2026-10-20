---
id: CES-58
title: One modern lint stack
---

## Directive

Python code uses one toolchain: ruff (format and lint), pyrefly (types) and botica (the rules that
can be checked deterministically), all run through prek. The legacy stack is not reintroduced, and
a legacy tool found in `pyproject.toml` or `prek.toml` is replaced:

| Legacy | Use |
|---|---|
| `black`, `yapf`, `autopep8` | `ruff format` |
| `isort` | `ruff` (import sorting, `I`) |
| `flake8` + plugins | `ruff` lint rules |
| `pyupgrade` | `ruff` (`UP`) |
| `pylint` | `ruff` + `pyrefly` |
| mypy (as the only checker) | `pyrefly` |

The type checker stays on. No setting or directive turns it off for a whole file, a project or a
build: `# pyrefly: ignore-errors`, `// @ts-nocheck`, `"strict": false` or an option of the strict
family set to `false` in `tsconfig.json`, and `typescript.ignoreBuildErrors` in `next.config`. A
type error that cannot be fixed yet is silenced on its line, with the reason.

## Why

- ruff covers black, isort, flake8, pyupgrade and most of pylint in one tool with one
  configuration. The old tools on top of it make overlapping passes and spread the configuration
  across five files.
- A checker that is off for a file reports nothing there, and the errors it would report stay in
  the code. An agent whose build fails on types can turn the checker off instead of fixing them.
