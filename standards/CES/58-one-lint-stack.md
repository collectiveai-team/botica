---
id: CES-58
title: One modern lint stack
---

## Directive

Python code uses one toolchain: ruff (format and lint), pyrefly (types) and botica (the rules that
can be checked deterministically), all run through prek. The legacy stack is not reintroduced:
`black`, `isort`, `flake8` (and its plugins), `pylint`, `autopep8`, `yapf` and standalone
`pyupgrade`.

The type checker stays on. No setting or directive turns it off for a whole file, a project or a
build: `# pyrefly: ignore-errors`, `// @ts-nocheck`, `"strict": false` or an option of the strict
family set to `false` in `tsconfig.json`, and `typescript.ignoreBuildErrors` in `next.config`. A
type error that cannot be fixed yet is silenced on its line, with the reason.

## Why

- ruff covers black, isort, flake8, pyupgrade and most of pylint in one fast tool with one config.
  Stacking the old tools on top means overlapping, conflicting passes and a config spread across
  five files.
- One toolchain means one configuration and one formatter.
- A checker that is off for a file reports nothing there, and the errors it would report stay in
  the code. An agent whose build fails on types can turn the checker off instead of fixing them.

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

## Migration

A repository where the checker is off somewhere turns it back on in its own pull request:

1. Remove the directive or the setting, and run the checker (`pyrefly check`, `tsc --noEmit`).
2. Fix the errors it reports. An error that has to stay gets a suppression on its line with the
   reason: `# pyrefly: ignore[<code>]`, `// @ts-expect-error <reason>`.
3. When there are too many errors for one pull request, keep the file in the `extend-ignores` of
   `[rules.no-disabled-typecheck]` and remove it from there file by file. In pyrefly, a
   `baseline` file holds the existing errors and reports only new ones.
