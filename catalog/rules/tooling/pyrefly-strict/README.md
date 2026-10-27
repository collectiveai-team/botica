## Directive

pyrefly checks the Python code with the `strict` preset, and an `Any` written in an annotation is
an error. The preset reports a parameter without an annotation, an empty container whose type
pyrefly cannot infer, a lambda whose parameters have no type, a generic without its arguments
(the `implicit-any` kinds), and a method that overrides another without `@override`. A target of
`mock.patch` that the module does not have (`missing-attribute-patch-target`) and a call of an
`async` function whose result nothing awaits (`unused-coroutine`) are errors too. botica checks
that `pyproject.toml` holds that configuration; pyrefly reports the code.

## Why

pyrefly gives the type `Any` to a parameter without an annotation, and checks nothing that flows
from it: `def handle(payload):` turns off the checks of every value that `payload` reaches, and
`pyrefly check` still reports 0 errors. An annotation `x: Any` does the same and says so. With the
default configuration neither is an error, so an agent can make a type error go away by removing
an annotation.

`mock.patch("app.billing.charge")` after `charge` is renamed patches nothing that the code calls,
and the test exercises the real function. pyrefly reports the target as a warning, which
`pyrefly check` prints only with `--min-severity warn` and which leaves its exit code at 0.
`refresh()` without `await` inside an `async def` creates a coroutine that never runs; pyrefly
reports it as an error by default, and the fragment keeps a project from lowering it without a
reason (`external-config-weakened`).

## Exceptions

The tests take fixtures of pytest (`capsys`, `tmp_path`) as parameters without annotations. A
`[[tool.pyrefly.sub-config]]` for `tests/**` turns off `implicit-any-parameter` there and keeps the
other kinds.

## Tool

pyrefly, the type checker of `base-python`, run by the `pyrefly` hook of prek
(<https://pyrefly.org/en/docs/configuration/#preset-strict>). The names of the error kinds are
those of pyrefly 1.4.0; pyrefly 0.55.0 stops with `unknown variant implicit-any-parameter` on the
`sub-config` below. The hook of `base-python` runs `uvx pyrefly` at the version of the parameter
`pyrefly-version`, the latest release when it is empty, and `required-version` below stops a
version outside its range.

## Configuration

In `pyproject.toml`, beside the other keys of `[tool.pyrefly]`:

```toml
[tool.pyrefly]
preset = "strict"
required-version = ">=1.4,<2"

[tool.pyrefly.errors]
explicit-any = true
missing-attribute-patch-target = "error"
unused-coroutine = "error"

[[tool.pyrefly.sub-config]]
matches = "tests/**"

[tool.pyrefly.sub-config.errors]
implicit-any-parameter = false
```

`missing-attribute-patch-target` takes `"error"` and not `true`: `true` turns a kind on at its
default severity, and the default of this kind is a warning, so `external-config-weakened` reports
`missing-attribute-patch-target = true`.

`external-config` checks `preset = "strict"` and the three kinds of `errors` where pyrefly reads
them, in `pyrefly.toml` or in `[tool.pyrefly]` of `pyproject.toml`. `preset = "all"` checks what
`strict` checks and holds it, and a kind of error that the repository sets under `errors`, to any severity,
is its choice. `required-version` makes pyrefly stop with an error when the hook runs a version
outside the range, so an old pyrefly fails with a message that names the version.

A repository with many errors adopts the preset in steps: it sets the kinds with the most errors to
`"warn"` in `[tool.pyrefly.errors]` (`implicit-any-parameter = "warn"`) and sets each back to
`true` when its errors are fixed.

## Do

```python
def total(items: list[Item]) -> Decimal:
    return sum((item.price for item in items), Decimal(0))


def parse(raw: object) -> Order:
    return Order.model_validate(raw)
```

## Don't

```python
def total(items):
    return sum(item.price for item in items)


def parse(raw: Any) -> Order:
    return Order.model_validate(raw)
```

## Suppression

`# pyrefly: ignore[<kind>]` on the line, with the reason after it:
`# pyrefly: ignore[explicit-any] -- the payload of the webhook is validated by the schema below`.
A whole file does not turn the checker off (`no-disabled-typecheck`).
