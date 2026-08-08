---
title: Use get_logger(__name__) — stdlib logging wrapped with RichHandler
section: log
scope: general
applies-to: all
status: current
tags: logging, rich, get_logger, convention
---

## Use get_logger(__name__) — stdlib logging wrapped with RichHandler

Ship a shared `get_logger(name)` factory: it calls `logging.basicConfig` with `RichHandler`,
format `"%(message)s"`, datefmt `"[%X]"`, and level read from the `LOG_LEVEL` env var
(default `"info"`). Call it once at module top with `__name__`; the returned standard
`logging.Logger` is used everywhere below. Do not create a second `basicConfig` call or a
bare `logging.getLogger`.

**Prefer:**

```python
# any_module.py
from mypackage.logger import get_logger

logger = get_logger(__name__)

def process(item: str) -> None:
    logger.info(f"processing {item}")
```

```python
# mypackage/logger/logger.py  — the factory
import os
import logging
from rich.logging import RichHandler

LOG_LEVEL = os.getenv("LOG_LEVEL", "info").lower()
LOG_LEVEL_MAP = {
    "debug": logging.DEBUG,
    "info": logging.INFO,
    "warning": logging.WARNING,
    "error": logging.ERROR,
}

def get_logger(name: str) -> logging.Logger:
    logging.basicConfig(
        level=LOG_LEVEL_MAP[LOG_LEVEL],
        format="%(message)s",
        datefmt="[%X]",
        handlers=[RichHandler(markup=True)],
    )
    return logging.getLogger(name)
```

**Avoid:**

```python
import logging
logging.basicConfig()                    # no Rich, wrong format
logger = logging.getLogger("hardcoded")  # loses __name__ hierarchy
```

Reference: `log-no-print` (no bare print in library code); `log-observability` (Logfire
layer added on top for production services).
