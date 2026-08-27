---
title: Libraries log, they don't print
section: log
scope: general
applies-to: all
status: current
tags: logging, print, anti-pattern
---

## Libraries log, they don't print

Library and service code must use `get_logger(__name__)` for all observability output.
Bare `print()` calls bypass log level filtering, structured output, and Logfire
instrumentation, and they pollute stdout in ways callers cannot suppress. Reserve
`print()` for CLI entrypoints and standalone scripts where a human is reading raw
terminal output.

**Prefer:**

```python
from mypackage.core.logger import get_logger

log = get_logger(__name__)

def run_task(name: str) -> None:
    log.info("task_started", task=name)
    try:
        _execute(name)
    except Exception:
        log.exception("task_failed", task=name)
        raise
```

**Avoid:**

```python
def run_task(name: str) -> None:
    print(f"Running task: {name}")    # silences under --quiet, breaks log capture
    try:
        _execute(name)
    except Exception as exc:
        print(f"Error: {exc}")        # no stack trace, no level, no filtering
```

Reference: `log-get-logger-structlog` for the factory; CLI entrypoints are the one place
`print` / `typer.echo` is appropriate.
