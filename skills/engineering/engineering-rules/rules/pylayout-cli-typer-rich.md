---
title: CLI entrypoints use a typed framework + Rich; libraries log instead of printing
section: pylayout
scope: general
applies-to: all
status: current
tags: cli, typer, cyclopts, rich, logging, entrypoints
---

## CLI entrypoints use a typed framework + Rich; libraries log instead of printing

User-facing command-line entrypoints are built with a **typed, declarative framework**
paired with `Rich` for output. What is flagged is manual, imperative argument plumbing —
`argparse`, `click`, raw `sys.argv` — not the choice of framework.

`Typer` and `Cyclopts` are both house-approved; `pydantic-settings` covers the
config-shaped cases. Cyclopts derives commands from plain signatures and needs no
decorator on every parameter, so it suits a CLI whose arguments are already typed.
Follow whichever the repo already uses.

Libraries and services never write to stdout directly — they log via the house logger.
This keeps library output invisible to callers unless they configure a handler.

**Prefer (CLI entrypoint, Typer):**

```python
import typer
from rich.console import Console

app = typer.Typer()
console = Console()

@app.command()
def stream_audio(host: str = "localhost", port: int = 8765) -> None:
    console.print(f"[green]Connecting to {host}:{port}[/green]")
    ...
```

**Prefer (CLI entrypoint, Cyclopts):**

```python
from cyclopts import App
from rich.console import Console

app = App()
console = Console()

@app.command
def stream_audio(host: str = "localhost", port: int = 8765) -> None:
    console.print(f"[green]Connecting to {host}:{port}[/green]")
    ...
```

**Prefer (library code):**

```python
from mypackage.core.logger import get_logger

log = get_logger(__name__)

def encode(text: str) -> list[float]:
    log.debug("text_encoding_started", length=len(text))
    ...
```

**Avoid:**

```python
# In a library — direct stdout pollutes the caller's output
def encode(text: str) -> list[float]:
    print(f"Encoding: {text[:50]}")   # don't do this in library code
    ...

# Imperative argument plumbing in an entrypoint
import sys
host = sys.argv[1] if len(sys.argv) > 1 else "localhost"
```

Cross-ref: `log-no-print` for the logging rule on why libraries must not print;
`log-get-logger-structlog` for the factory `get_logger` comes from.
