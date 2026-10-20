## Directive

Build command-line interfaces with a typed, declarative framework (Typer or Cyclopts) and Rich for
the output. The rule reports `argparse`, `click` and `sys.argv` handled by hand. Reading
`sys.argv[0]` and handing `sys.argv` (whole, or from index 1) to a call are the exceptions of
CES-67, and the rule leaves them alone. A warning by default.

Typer and Cyclopts are both approved; follow whichever the repository already uses. Cyclopts
derives its commands from plain signatures and needs no decorator on every parameter, so it suits
a CLI whose arguments are already typed.

## Why

- A typed framework derives the parser from the signatures and type hints of the functions, so
  arguments, options, help and validation stay in step with the code.
- `argparse`, `click` and hand-written `sys.argv` parsing say the same thing a second time, drift
  out of date and lose the types a checker could use.

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
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("name")
args = parser.parse_args(sys.argv[1:])
```

## Suppression

A comment on the line of the import or of the `sys.argv` access silences it.

```python
import argparse  # botica: ignore[cli-typed-framework]
import argparse  # noqa: BTCLI001 -- reason
```
