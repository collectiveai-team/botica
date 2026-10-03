# CES-67 · typed, declarative CLIs

**Code:** `CES-67` &nbsp;·&nbsp; **Slug:** `cli-typed-framework` &nbsp;·&nbsp; **Enforced by:**
ast-grep (`warning`) &nbsp;·&nbsp; **Tracker:**
[#67](https://github.com/collectiveai-team/botica/issues/67)

## Directive

Build command-line interfaces with a **typed, declarative framework** — [Typer], [Cyclopts], or
`pydantic-settings` — paired with [Rich] for output. The rule warns on `argparse`, `click`, and
raw `sys.argv` access. The framework choice is open; what's flagged is manual, imperative
argument plumbing.

[Typer]: https://typer.tiangolo.com
[Cyclopts]: https://cyclopts.readthedocs.io
[Rich]: https://rich.readthedocs.io

## Why

- Typed frameworks derive the parser from function signatures and type hints: arguments,
  options, help, and validation come for free and stay in sync with the code.
- `argparse`/`click` and hand-rolled `sys.argv` parsing duplicate that information imperatively,
  drift out of date, and lose the static types a checker could use.

## Severity is a warning

This is a `warning`, not an error: modern tooling is **encouraged, not mandated**. An existing
`argparse`/`click` CLI is not a build-breaker — the warning is a nudge to adopt the house
pattern on the next touch.

## Do

```python
import cyclopts

app = cyclopts.App()

@app.command
def greet(name: str, *, loud: bool = False) -> None:
    ...
```

## Don't

```python
import argparse           # cli-typed-framework

p = argparse.ArgumentParser()
p.add_argument("name")
args = p.parse_args(sys.argv[1:])   # cli-typed-framework
```

## Suppressing (rare, must be visible)

When a dependency forces `argparse`/`click` (e.g. extending a click-based third-party CLI), keep
the import and add a visible ignore:

```python
import click  # ast-grep-ignore: cli-typed-framework
```

The slug is the stable suppression key; the `CES-67` code in the message is for humans.
