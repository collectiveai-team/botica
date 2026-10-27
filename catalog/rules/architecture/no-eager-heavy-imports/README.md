## Directive

In a project whose `requires-python` is 3.15 or later, a heavy library is imported at the top of
the module with `lazy import` or `lazy from` (PEP 810). The rule reports an `import torch`,
`import torch.nn` or `from torch import nn` without `lazy` at the top of a module, or in an `if` or
`with` block at the top. The heavy libraries are the option `modules`: `torch`, `tensorflow`, `jax`,
`transformers` and `cv2` by default, and a submodule of one of them. A warning by default.

The rule does not report:

- `lazy import torch` and `lazy from torch import nn`;
- an import of a module that a `__lazy_modules__` at the top of the module names before the import,
  with its full dotted name (`"torch.nn"` for `import torch.nn`);
- an import in the body of `if TYPE_CHECKING:` or `if typing.TYPE_CHECKING:`;
- an import in a `try` block, where Python 3.15 does not accept `lazy`, such as an optional
  dependency in `try: import cv2 / except ImportError:`;
- an import inside a function or a class body, which ruff `PLC0415` reports (`ruff-imports-at-top`);
- a relative import.

The rule has `python = ">=3.15"`: in a project whose `requires-python` allows an older version, or
names none, it is not active, and `ruff-heavy-imports-in-function` asks for the import inside the
function instead. `base-python` selects the three rules.

## Why

Importing a module runs the imports at its top. `import torch` takes from one to several seconds
and about 500 MB of memory (torch 2.13 on Linux), and every program that imports the module pays
for it: `--help` of a command-line tool, a test that never touches the model, the start of an API
whose routes do not all use it. `lazy import` keeps the import at the top, where the reader sees
it, and loads the library at its first use.

ruff `TID253` (`banned-module-level-imports`) reports the eager imports, but ruff 0.17.1 also
reports `lazy import torch` and an import that `__lazy_modules__` makes lazy.

## Exceptions

The test files (`**/test_*.py`, `**/*_test.py`) and stub files (`*.pyi`) are left out. A
`conftest.py` is not: one at the root that imports torch loads it for every run of the tests. The
adapters of `impl` are not left out either, since `lazy` costs them nothing.

The rule reads `.py` files; it does not read notebooks (`.ipynb`).

The grammar of tree-sitter-python that botica uses does not parse `lazy` yet. The rule recognises
`lazy` written right before `import` or `from` on the same line.

## Do

```python
lazy import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```

```python
__lazy_modules__ = ["torch"]

import torch
```

## Don't

```python
import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```

## Options

- `modules` (strings, default `["torch", "tensorflow", "jax", "transformers", "cv2"]`): the heavy
  libraries. A name also covers its submodules, so `"torch"` covers `torch.nn`.

## Suppression

`# botica: ignore[no-eager-heavy-imports]` on the line of the import, with the reason, such as a
library whose import has to run its side effects at once.
