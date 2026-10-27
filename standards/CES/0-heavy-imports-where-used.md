---
id: CES-0
title: A heavy library is imported where it is used
---

## Directive

A Python module does not load a heavy library, such as `torch`, `tensorflow`, `jax`,
`transformers` or `cv2`, when it is imported, unless the module is an adapter that exists to use
that library.

- A module outside the adapters imports the library inside the function that uses it.
- On Python 3.15 or later the import can stay at the top as `lazy import torch` or
  `lazy from transformers import AutoModel` (PEP 810), which loads the library at its first use.
- Code that also runs on older versions names the library in `__lazy_modules__` at the top of the
  module, before the import: Python 3.15 makes that import lazy and older versions leave it eager.
- An import that only type annotations need goes under `if TYPE_CHECKING:`.

The adapters live in the `impl` package (CES-16), and their modules import the library at the top.
Tests import it where they need it.

## Why

Importing a module runs the imports at its top. `import torch` of torch 2.13 on Linux took 8.3
seconds the first time and 1.0 second after, and took the memory of the process from 12 MB to
518 MB. Each program that imports the module pays for it, even when it never calls the model: `--help` of a command-line tool, a test of code that does not touch the model, the start of
an API where few routes use it. PEP 810 gives this cost as the reason for lazy imports: "even
running the command with --help can load dozens of unnecessary modules and take several seconds."

ruff `TID253` (`banned-module-level-imports`) reports an import of a listed module at the top of a
module, but ruff 0.17.1 with `target-version = "py315"` also reports `lazy import torch` and an
import that `__lazy_modules__` makes lazy, which are the fixes this standard asks for.
`no-eager-heavy-imports` reports the eager imports and leaves those two alone.

## Examples

```python
# Do
def embed(texts: list[str]) -> list[list[float]]:
    import torch

    with torch.no_grad():
        return model(texts).tolist()


# Don't: every importer of this module loads torch
import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```
