# CES-58 · one modern lint stack

**Code:** `CES-58` &nbsp;·&nbsp; **Slug:** `py-legacy-lint-stack` &nbsp;·&nbsp; **Tier:** judgment
&nbsp;·&nbsp; **Tracker:** [#58](https://github.com/collectiveai-team/botica/issues/58)

## Directive

Use the house Python toolchain — **ruff** (format + lint), **pyrefly** (types), **ast-grep**
(structural rules), all run through **prek**. Don't reintroduce the legacy stack: `black`,
`isort`, `flake8` (+ plugins), `pylint`, `autopep8`, `yapf`, standalone `pyupgrade`.

## Why

- ruff subsumes black, isort, flake8, pyupgrade, and most of pylint in one fast tool with one
  config. Stacking the old tools on top means overlapping, conflicting passes and a config spread
  across five files.
- One toolchain = one source of truth for "what clean looks like", faster CI, and no fights
  between formatters.

## Replacements

| Legacy | House |
|---|---|
| `black`, `yapf`, `autopep8` | `ruff format` |
| `isort` | `ruff` (import sorting, `I`) |
| `flake8` + plugins | `ruff` lint rules |
| `pyupgrade` | `ruff` (`UP`) |
| `pylint` | `ruff` + `pyrefly` |
| mypy (as the only checker) | `pyrefly` |

## Judgment

No linter forbids adding `black` — this is a judgment call. If you find a legacy tool creeping
into `pyproject.toml` or `prek.toml`, remove it and express the intent through the house stack
instead.
