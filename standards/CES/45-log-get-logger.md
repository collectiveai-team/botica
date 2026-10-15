---
id: CES-45
title: Get loggers from the house get_logger
---

## Directive

Get a logger with `get_logger(__name__)`, the house function of CES-74, and never with
`logging.getLogger(...)` directly. The standard fixes the entry point, not the backend: how
`get_logger` is configured is the business of CES-74.

## Why

One entry point makes every module log the same way. Scattered `getLogger` calls drift into
inconsistent formats and lost context.
