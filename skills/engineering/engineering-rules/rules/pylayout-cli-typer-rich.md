---
title: CLI entrypoints use Typer + Rich; libraries log instead of printing
section: pylayout
scope: general
applies-to: all
status: current
tags: cli, typer, rich, logging, entrypoints
---

## CLI entrypoints use Typer + Rich; libraries log instead of printing

User-facing command-line entrypoints are built with `Typer` (argument parsing) and
`Rich` (output formatting). Libraries and services never write to stdout directly —
they log via the logger. This keeps library output invisible to callers unless they
configure a log handler.

**Prefer (CLI entrypoint):**

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

**Prefer (library code):**

```python
import logging

log = logging.getLogger(__name__)

def encode(text: str) -> list[float]:
    log.debug("encoding text", extra={"length": len(text)})
    ...
```

**Avoid:**

```python
# In a library — direct stdout pollutes the caller's output
def encode(text: str) -> list[float]:
    print(f"Encoding: {text[:50]}")   # don't do this in library code
    ...
```

Cross-ref: `log-no-print` for the logging rule on why libraries must not print.
