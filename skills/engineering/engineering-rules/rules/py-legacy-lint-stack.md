---
title: Migrate a legacy black + isort + pylint + mypy stack to the house stack
section: py
scope: general
applies-to: python
status: current
tags: python, black, isort, pylint, flake8, mypy, formatting, linting, migration
---

## Migrate a legacy black + isort + pylint + mypy stack to the house stack

A legacy lint stack uses `black` (88-char lines) + `isort` (profile=black) + `pylint` or
`flake8`, with `mypy` as the only type checker and `pre-commit` as the runner. When you
encounter this setup, plan a migration to the house stack — `ruff` + `ruff-format` +
`pyrefly` + `ast-grep`, run through `prek` (see `py-ruff-format-modern`, CES-58). Don't run
ruff alongside black — they conflict on line length and quote style. Migrate as an explicit,
standalone task, never as a side effect of feature work.

**Recognizing a legacy setup:**

```toml
# pyproject.toml — legacy indicators
[tool.pylint.messages_control]
disable = "C0330, C0326"

[tool.isort]
profile = "black"
length_sort = true
combine_as_imports = true
force_sort_within_sections = true
# black default: line-length = 88

[tool.mypy]           # mypy as the only checker — pyrefly is the house checker
python_version = "3.11"
```

```ini
# tox.ini or setup.cfg — flake8 variant
[flake8]
max-line-length = 88
select = C,E,F,W,B,B950
extend-ignore = E203,E501
```

A `.pre-commit-config.yaml` is the runner-level tell: hooks are declared in `prek.toml` here.

**Replacements:**

| Legacy | House |
|---|---|
| `black`, `yapf`, `autopep8` | `ruff format` |
| `isort` | `ruff` (import sorting, `I`) |
| `flake8` + plugins | `ruff` lint rules |
| `pyupgrade` | `ruff` (`UP`) |
| `pylint` | `ruff` + `pyrefly` |
| `mypy` (as the only checker) | `pyrefly` |
| `pre-commit` (runner) | `prek` |

**Avoid while the legacy stack is still in place:**

```toml
# Don't add ruff or ruff-format without completing the migration
[tool.ruff]
line-length = 100   # conflicts with the existing black 88-char baseline
```

**Migration path:**

1. Remove `black`, `isort`, `pylint`/`flake8`, `mypy` from deps and from the hook config.
2. Add the ruff + pyrefly config and a `prek.toml` (see `py-ruff-format-modern`); delete
   `.pre-commit-config.yaml` once its hooks are ported.
3. Run `uv run ruff format .` once to reformat the whole repo in a single dedicated commit.
4. Fix any lint errors surfaced by ruff, then the type errors surfaced by pyrefly.
5. Update CI to run `uvx prek run --all-files` instead of the old tools.
