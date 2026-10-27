## Directive

In a project whose `requires-python` allows a version before 3.15 (`python = "<3.15"`), ruff
reports `torch`, `tensorflow`, `jax`, `transformers` and `cv2` imported at the top of a module,
with `TID253` (`banned-module-level-imports`), and the module imports them inside the function that
uses them. `PLC0415` (`import-outside-top-level`) stays off, because it reports the imports that
this rule asks for. From 3.15 the rule is not active, and `ruff-imports-at-top` and
`no-eager-heavy-imports` ask for `lazy import` instead.

## Why

Importing a module runs the imports at its top, and `import torch` takes from one to several
seconds and about 500 MB of memory (torch 2.13 on Linux). Every program that imports the module
pays it: `--help` of a command-line tool, a test that never touches the model, the start of an API.
Before Python 3.15 there is no lazy import in the language, and an import inside the function is
the way to load the library only when it is used.

## Exceptions

The adapters, which exist to use the library, and the test files import it at the top, through
`per-file-ignores`. The adapters are a suggestion of the project and not part of the rule: the
section of ruff of `base-python` writes the glob of its parameter `ruff-adapters`, `**/impl/**` by
default (`--set 'ruff-adapters=coro/backends/**'` at install), and a project adds more folders by
hand. A `conftest.py` is not left out: one at the root that imports torch loads it for every run of
the tests.

## Tool

[ruff](https://docs.astral.sh/ruff/rules/banned-module-level-imports/) checks the code, through
the hook `ruff-check` of `prek.toml`. In a project that has `pyproject.toml`, `ruff.toml` or
`.ruff.toml`, `external-config` reads the settings of ruff where ruff reads them and reports the
settings that do not select `TID253` or lack a module of the list in
`banned-module-level-imports`. It does not check `per-file-ignores`, which the project decides, nor
that `PLC0415` is off.

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

`base-python` writes this configuration: its parameter `ruff-import-rule` is `TID253` by default,
and the list and the three globs are in its section of ruff.

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
