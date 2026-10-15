## Directive

Tests live in a `tests/` directory **beside** the package, not in it. Inside a Python package, a
Python file under a `tests/` or `test/` directory, a `test_*.py` or `*_test.py` file and a
`conftest.py` are errors. A non-Python file under `tests/` (the end-to-end tests of a web app that
ships inside the package, say) is not.

## Why

- It is the layout `pytest` recommends for applications ("tests outside application code"): the
  tests run against the installed package, and a `src/` layout avoids accidental imports.
- It is what Collective's repositories do: 69 of the 75 that have a Python package keep their tests
  outside it. The ones that mix them in are the exception this rule points at.
- Projects that ship their tests with the package on purpose (numpy, scipy, scikit-learn do) are
  libraries; exclude them with `ignores`.

## What the package is

The directories of code of the project (see the `code-dirs` setting): for Python, a directory with
an `__init__.py` inside the project or inside its `src/`. `tests`, `test`, `src`, `docs`,
`scripts`, `examples`, `fixtures` and `benchmarks` are never a package. Other ecosystems are not covered: Go and Rust put
their tests with the code by design, and TypeScript projects are split.

## Names that look like tests

The rule goes by the names `pytest` collects, so a module that is code but is called
`connection_test.py` or `test_fire_action.py` is reported. Exclude it with `ignores`.

## Suppression

The finding is about a whole file and has no line, so it cannot be silenced with a line comment.
Exclude the path with `ignores` or `extend-ignores` in the rule's configuration, or exclude it in
the project configuration.
