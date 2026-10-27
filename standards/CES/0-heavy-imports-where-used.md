---
id: CES-0
title: A heavy library loads when it is used
---

## Directive

A Python module does not load a heavy library, such as `torch`, `tensorflow`, `jax`,
`transformers` or `cv2`, when it is imported. How it defers the import depends on the lowest
version of Python that the `requires-python` of the project allows; a project whose
`requires-python` sets no lower bound counts as before 3.15 (ADR 0015).

| `requires-python` | Where the import goes | ruff | botica | `ruff-import-rule` of `base-python` |
|---|---|---|---|---|
| allows a version before 3.15, or sets no lower bound | inside the function that uses the library | `TID253` on, with the list in `banned-module-level-imports`; `PLC0415` off | `ruff-heavy-imports-in-function` | `TID253`, the default |
| 3.15 or later | at the top of the module, as `lazy import torch` or `lazy from transformers import AutoModel` (PEP 810) | `PLC0415` on, the tests included; `TID253` off | `ruff-imports-at-top`, `no-eager-heavy-imports` | `PLC0415` |

`base-python` selects the three rules, and each one runs only in its range.

Before 3.15, the adapters, which exist to use the library, import it at the top. Which folders hold
them is the choice of the project: `base-python` suggests the `impl` package (CES-16) through its
parameter `ruff-adapters`, and a project adds its own. From 3.15 the adapters write `lazy` too,
which costs them nothing, and a module may name the library in `__lazy_modules__` at its top,
before the import, instead of writing `lazy`.

In both ranges:

- an import that only type annotations need goes under `if TYPE_CHECKING:`;
- the test files (`test_*.py`, `*_test.py`) import the library at the top, and `conftest.py`
  follows the rule, since one at the root loads the library for every run of the tests.

From 3.15, an optional dependency imported in `try: ... except ImportError:` stays eager, because
Python 3.15 does not accept `lazy` in a `try` block.

## Why

Importing a module runs the imports at its top. `import torch` of torch 2.13 on Linux took 8.3
seconds the first time and 1.0 second after, and took the memory of the process from 12 MB to
518 MB. Each program that imports the module pays for it, even when it never calls the model:
`--help` of a command-line tool, a test of code that does not touch the model, the start of an API
where few routes use it. PEP 810 gives this cost as the reason for lazy imports: "even running the
command with --help can load dozens of unnecessary modules and take several seconds."

Before 3.15 the language has no lazy import, and an import inside the function is how a module
loads the library only when it is used. ruff `PLC0415` reports every import inside a function, so
it stays off there. From 3.15, `lazy import` keeps every import at the top, where the reader sees
the dependencies of the module, and loads the library at its first use; an import inside a function
no longer saves anything, and `PLC0415` reports it. ruff 0.17.1 with `target-version = "py315"`
reports `lazy import torch` and an import that `__lazy_modules__` makes lazy under `TID253`, so
`TID253` stays off there and `no-eager-heavy-imports` reports the eager imports instead.

PEP 810 forbids `lazy` in a function, a class body and a `try` block, and allows it in `if` and
`with` blocks at the top of the module.

## Examples

```python
# Before 3.15. Do
def embed(texts: list[str]) -> list[list[float]]:
    import torch

    with torch.no_grad():
        return model(texts).tolist()


# From 3.15. Do
lazy import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()


# In both. Don't: every importer of this module loads torch
import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```
