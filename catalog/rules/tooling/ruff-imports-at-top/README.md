## Directive

In a project whose `requires-python` is 3.15 or later, ruff reports an import inside a function or
a class body with `PLC0415` (`import-outside-top-level`), and every import goes at the top of the
module. A heavy library is imported there with `lazy import` (`no-eager-heavy-imports`).
`TID253` stays off: `banned-module-level-imports` is empty, because ruff 0.17.1 reports
`lazy import torch` and an import that `__lazy_modules__` makes lazy.

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
hook `ruff-check` of `prek.toml`. `external-config` reports the settings that neither select nor
ignore `PLC0415`. It does not check that `banned-module-level-imports` is empty.

## Configuration

```toml
[tool.ruff.lint]
select = ["PLC0415"]
```

The section of ruff that `base-python` installs does not select `PLC`: add the code to its
`select`. The recipe `python-lazy-imports` selects this rule and `no-eager-heavy-imports`.

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
