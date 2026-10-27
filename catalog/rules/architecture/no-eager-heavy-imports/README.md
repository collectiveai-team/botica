## Directive

A Python module does not import a heavy library when it loads. The rule reports an `import torch`,
`import torch.nn` or `from torch import nn` that runs when the module is imported: at the top of the
module, in a class body, or in an `if`, `try` or `with` block at the top. The heavy libraries are
the option `modules`: `torch`, `tensorflow`, `jax`, `transformers` and `cv2` by default, and a
submodule of one of them. A warning by default.

The rule does not report:

- an import inside a function or a method;
- an import in the body of `if TYPE_CHECKING:` or `if typing.TYPE_CHECKING:`;
- `lazy import torch` and `lazy from torch import nn` (Python 3.15, PEP 810);
- an import of a module that a `__lazy_modules__` at the top of the module names before the import,
  with its full dotted name (`"torch.nn"` for `import torch.nn`), unless the import is in a `try`
  block or a class body, where Python 3.15 keeps it eager;
- a relative import.

## Why

Importing a module runs the imports at its top. `import torch` takes from one to several seconds
and about 500 MB of memory (torch 2.13 on Linux), and every program that imports the module pays for it: `--help` of a command-line tool,
a test that never touches the model, the start of an API whose routes do not all use it. An import
inside the function that uses the library loads it only when that function runs. On Python 3.15,
`lazy import` keeps the import at the top and loads the library at its first use; `__lazy_modules__`
does the same on 3.15 and leaves the import eager on older versions, so code that supports both can
declare it now.

ruff `TID253` (`banned-module-level-imports`) reports the same imports, but it also reports
`lazy import torch` and an import that `__lazy_modules__` makes lazy.

## Exceptions

An adapter exists to use the library, and its module may import it at the top. The rule leaves out
the `impl` packages (`**/impl/**`), where adapters live, and the tests (`**/tests/**`,
`test_*.py`, `*_test.py`, `conftest.py`), which import the library to test those adapters. It also
leaves out stub files (`*.pyi`), which never run. A package that imports all its adapters in
`impl/__init__.py` still loads the library for every importer of `impl`.

The rule reads `.py` files; it does not read notebooks (`.ipynb`).

The grammar of tree-sitter-python that botica uses does not parse `lazy` yet. The rule recognises
`lazy` written right before `import` or `from` on the same line.

## Do

```python
def embed(texts: list[str]) -> list[list[float]]:
    import torch

    with torch.no_grad():
        return model(texts).tolist()
```

```python
lazy import torch
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

`# botica: ignore[no-eager-heavy-imports]` on the line of the import, with the reason. A folder of
adapters with another name goes in the `extend-ignores` of `[rules.no-eager-heavy-imports]`.
