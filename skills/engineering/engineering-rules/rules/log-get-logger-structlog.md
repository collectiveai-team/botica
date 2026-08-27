---
title: Use get_logger(__name__) — the house structlog factory
section: log
scope: general
applies-to: all
status: current
tags: logging, structlog, get_logger, convention
---

## Use get_logger(__name__) — the house structlog factory

Logging is configured in exactly one place: a `core/logger.py` module built on
[structlog](https://www.structlog.org). Import `get_logger` from it and call it once at module
top with `__name__`. Never call `logging.getLogger(...)` directly, and never add a second
`logging.basicConfig`.

Rendering follows the environment — structured JSON to stdout under `ENV=prod`/`production`,
colored console rendering otherwise. The level is read once from `LOG_LEVEL` (default `INFO`,
case-insensitive), and the stack configures lazily on the first `get_logger` call.

Emit **events as key/value pairs**, not pre-formatted strings: that is what keeps a log
queryable once it reaches a sink. Bind recurring context once with `.bind()` rather than
threading it through every call site.

**Prefer:**

```python
# any_module.py
from mypackage.core.logger import get_logger

log = get_logger(__name__)

def process(item_id: str, request_id: str) -> None:
    bound = log.bind(request_id=request_id)
    bound.info("item_processing_started", item_id=item_id)
```

**Avoid:**

```python
import logging
from rich.logging import RichHandler

logging.basicConfig(handlers=[RichHandler(markup=True)])  # a second config point
logger = logging.getLogger("hardcoded")                   # bypasses the house factory
logger.info(f"processing {item_id}")                      # a string, not a queryable event
```

Rich remains the house presentation layer for CLI output (`pylayout-cli-typer-rich`); it is
not the logging handler.

Reference: the canonical factory ships as a drop-in snippet, `.agents/snippets/core/logger.py`
(CES-74 · `core-logger`), which `log-get-logger` (CES-45) and `log-no-print` (CES-46) point at.
See also `log-observability` (Logfire layer added on top for production services).
