## Directive

In a project whose `requires-python` is 3.15 or later (`python = ">=3.15"`), ruff reports an
import inside a function or a class body with `PLC0415` (`import-outside-top-level`), and every
import goes at the top of the module, the tests included. A heavy library is imported there with
`lazy import` (`no-eager-heavy-imports`). `TID253` stays off, because ruff 0.17.1 reports
`lazy import torch` and an import that `__lazy_modules__` makes lazy under it. Before 3.15 the rule
is not active, and `ruff-heavy-imports-in-function` asks for the import inside the function.

## Why

An import inside a function hides a dependency of the module from the reader of its top, and
Python resolves it again on each call. Python 3.15 has `lazy import` (PEP 810): the import stays at
the top and the library loads at its first use, which is what an import inside a function was for.

## Exceptions

`PLC0415` does not report an import in the body of `if TYPE_CHECKING:`, in a `try` block or in a
`with` block at the top of the module, so an optional dependency in
`try: import yaml / except ImportError:` stays where it is.

## Tool

[ruff](https://docs.astral.sh/ruff/rules/import-outside-top-level/) checks the code, through the
hook `ruff-check` of `prek.toml`. In a project that has `pyproject.toml`, `ruff.toml` or
`.ruff.toml`, `external-config` reports the settings that neither select nor ignore `PLC0415`. It
does not check that `TID253` is off.

## Configuration

```toml
[tool.ruff.lint]
select = ["PLC0415"]
```

`base-python` writes `PLC0415` in the `select` of its section of ruff when its parameter
`ruff-import-rule` is `PLC0415` (`botica recipe install base-python --set
ruff-import-rule=PLC0415`), and then selects no `TID253`; its default, `TID253`, is for a project
before 3.15.

## Do

```python
lazy import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```

## Don't

```python
def embed(texts: list[str]) -> list[list[float]]:
    import torch

    with torch.no_grad():
        return model(texts).tolist()
```

## Suppression

`# noqa: PLC0415` on the line of the import, with the reason, such as an import that has to run
after a setting of the process changes.
