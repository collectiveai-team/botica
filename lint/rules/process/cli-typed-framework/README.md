# cli-typed-framework · `BTPROC001`

**Standard:** CES-67 (documentation only) · **Family:** process · **Languages:** Python

## Directive

Build command-line interfaces with a typed, declarative framework (Typer, Cyclopts or
`pydantic-settings`) and Rich for the output. The rule reports `argparse`, `click` and raw
`sys.argv` access. The choice of framework is open; what it reports is argument handling written
by hand. A warning by default: modern tools are encouraged, not required.

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

Silence one line with `# botica: ignore[cli-typed-framework]` or `# noqa: BTPROC001`.
