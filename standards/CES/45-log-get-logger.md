---
id: CES-45
title: Get loggers from the house get_logger
---

## Directive

Get a logger with `get_logger(__name__)`, the function of CES-74, and not with
`logging.getLogger(...)` directly. CES-74 configures `get_logger`.

## Why

One entry point gives every module the same format and the same context.
