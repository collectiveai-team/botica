# log-no-print · `BTLOG002`

**Standard:** CES-46 (documentation only) · **Family:** logging · **Languages:** Python

## Directive

Don't use `print()` in importable library code. Emit through the house `get_logger`
(`.agents/snippets/core/logger.py`) so output is structured, leveled and routable. A warning by
default.

## Why

`print()` writes unstructured text to stdout, with no level, timestamp or context, and a library
that prints pollutes the stdout of every program that imports it. Logging lets the application
decide where output goes.

## What is exempt

Entrypoints, where stdout is the product: `__main__.py`, `cli.py`, anything under a `cli/`
directory, `conftest.py`, anything under `tests/`, and any `print()` inside an
`if __name__ == "__main__":` guard.

## Suppression

Silence one line with `# botica: ignore[log-no-print]` or `# noqa: BTLOG002`.
