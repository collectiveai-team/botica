## Directive

Tests live in a `tests/` directory beside the package, not in it. Inside a Python package, a
Python file under a `tests/` or `test/` directory, a `test_*.py` or `*_test.py` file and a
`conftest.py` are errors. A non-Python file under `tests/` (the end-to-end tests of a web app that
ships inside the package, say) is not.

The package is a directory of code of the project (see the `code-dirs` setting): for Python, a
directory with an `__init__.py` inside the project or inside its `src/`. The rule covers Python: Go
and Rust put their tests with the code by design.

## Why

- It is the layout `pytest` recommends for applications ("tests outside application code"): the
  tests run against the installed package, and a `src/` layout avoids accidental imports.
- Projects that ship their tests with the package on purpose (numpy, scipy, scikit-learn do) are
  libraries; exclude them with `ignores`.

## Exceptions

`tests`, `test`, `src`, `docs`, `scripts`, `examples`, `fixtures` and `benchmarks` are never a
package.

## Do

```text
myproject/
  src/myproject/
    __init__.py
    orders.py
  tests/
    conftest.py
    test_orders.py
```

## Don't

```text
myproject/
  src/myproject/
    __init__.py
    orders.py
    tests/
      test_orders.py
```

## Suppression

The finding belongs to a whole file. `extend-ignores` in the configuration of the rule leaves out
the files that match a glob. The rule goes by the names `pytest` collects, so a module of code
called `connection_test.py` or `test_fire_action.py` is reported, and `extend-ignores` leaves it
out.

```toml
# botica.toml
[rules.no-tests-in-package]
extend-ignores = ["**/connection_test.py"]
```
