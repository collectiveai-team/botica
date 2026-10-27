## Directive

In a project whose `requires-python` allows a version before 3.15, ruff reports `torch`,
`tensorflow`, `jax`, `transformers` and `cv2` imported at the top of a module, with `TID253`
(`banned-module-level-imports`), and the module imports them inside the function that uses them.
The adapters of the `impl` package and the test files import them at the top. `PLC0415`
(`import-outside-top-level`) stays off, because it reports the imports that this rule asks for.

## Why

Importing a module runs the imports at its top, and `import torch` takes from one to several
seconds and about 500 MB of memory (torch 2.13 on Linux). Every program that imports the module
pays it: `--help` of a command-line tool, a test that never touches the model, the start of an API.
Before Python 3.15 there is no lazy import in the language, and an import inside the function is
the way to load the library only when it is used.

## Exceptions

`**/impl/**` holds the adapters, which exist to use the library. `**/test_*.py` and `**/*_test.py`
are the test files that import it to test those adapters. A `conftest.py` is not left out: one at
the root that imports torch loads it for every run of the tests. A project whose adapters live in
another folder adds it to `per-file-ignores`.

## Tool

[ruff](https://docs.astral.sh/ruff/rules/banned-module-level-imports/) checks the code, through
the hook `ruff-check` of `prek.toml`. `external-config` reads its settings where ruff reads them
and reports the settings that do not select `TID253`, that lack a module of the list in
`banned-module-level-imports`, or that lack one of the three globs in `per-file-ignores`.
`external-config` does not check that `PLC0415` is off.

## Configuration

```toml
[tool.ruff.lint]
select = ["TID253"]

[tool.ruff.lint.flake8-tidy-imports]
banned-module-level-imports = ["torch", "tensorflow", "jax", "transformers", "cv2"]

[tool.ruff.lint.per-file-ignores]
"**/impl/**" = ["TID253"]
"**/test_*.py" = ["TID253"]
"**/*_test.py" = ["TID253"]
```

The section of ruff that `base-python` installs selects `TID`, which takes in `TID253`, and already
has the tables `[tool.ruff.lint.flake8-tidy-imports]` and `[tool.ruff.lint.per-file-ignores]`: add
the keys to them. The recipe `python-heavy-imports` selects this rule.

## Do

```python
def embed(texts: list[str]) -> list[list[float]]:
    import torch

    with torch.no_grad():
        return model(texts).tolist()
```

## Don't

```python
import torch


def embed(texts: list[str]) -> list[list[float]]:
    with torch.no_grad():
        return model(texts).tolist()
```

## Suppression

`# noqa: TID253` on the line of the import, with the reason. A folder goes in `per-file-ignores`.
