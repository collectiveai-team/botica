## Directive

ruff selects the families `ASYNC`, `A`, `BLE` and `SLOT`, and the codes `PLR0124` and `PLR0133`.
`ASYNC` reports a blocking call inside an `async def`, such as `time.sleep`, `open` or a call of
`requests`. `A` reports a variable, an argument, an attribute, an import or a module whose name
hides a builtin (`list`, `id`, `filter`) or a module of the standard library. `BLE` reports an `except Exception` or `except BaseException`
that neither raises again nor logs the traceback. `SLOT` reports a subclass of `str`, `tuple` or
`NamedTuple` without `__slots__`. `PLR0124` reports a comparison of a name with itself (`x == x`),
and `PLR0133` a comparison of two constants (`1 == 1`).

## Why

`E722` reports a bare `except:`, and `S110` and `S112` an `except` whose body is `pass` or
`continue`. An `except Exception:` that logs a warning and returns passes the three, and the error
it hides does not reach the caller. A blocking call inside an `async def` stops every other task of
the event loop while it runs, and no other family of the section reports it. A parameter named
`list` hides the builtin for the rest of the function. A comparison whose result is fixed is always
true or always false, and an `assert` that holds one passes whatever the code does.

## Tool

[ruff](https://docs.astral.sh/ruff/rules/) checks the code, through the hook `ruff-check` of
`prek.toml`. The selection of ruff enforces the rule, and `external-config` reads it where ruff
reads it: `.ruff.toml`, `ruff.toml` or `[tool.ruff]` of `pyproject.toml`, under `select` or
`extend-select`. It reports the settings that neither select nor ignore one of the families or
codes. A family that the repository switches off with `ignore` or `extend-ignore` is its decision,
and `external-config` does not report it. `external-config-weakened`, when it is selected, reports
it as a warning.

## Configuration

```toml
[tool.ruff.lint]
select = ["ASYNC", "A", "BLE", "SLOT", "PLR0124", "PLR0133"]
```

The section of ruff that `base-python` installs has them beside the other families. Before adding
them to an existing repository, `ruff check --select ASYNC,A,BLE,SLOT,PLR0124,PLR0133` lists what
they report.

## Do

```python
async def read(path: anyio.Path) -> str:
    return await path.read_text()


def filter_items(items: list[Item], predicate: Callable[[Item], bool]) -> list[Item]:
    return [item for item in items if predicate(item)]


try:
    charge(order)
except PaymentDeclined:
    logger.warning("payment declined", order_id=order.id)


assert order.total() == Decimal("12.50")
```

## Don't

```python
async def read(path: str) -> str:
    with open(path) as file:
        return file.read()


def filter_items(list, filter): ...


try:
    charge(order)
except Exception:
    logger.warning("payment failed")


total = order.total()
assert total == total
```

## Suppression

A comment of ruff on the line silences one finding, with the reason: `# noqa: BLE001` on the
`except Exception` of the loop of a worker that has to go on with the next job. A module whose name
is that of a module of the standard library goes in `allowed-modules` of
`[tool.ruff.lint.flake8-builtins]`. A family or a code that the repository does not want goes in
`ignore` of `[tool.ruff.lint]`, and this rule does not report it.
