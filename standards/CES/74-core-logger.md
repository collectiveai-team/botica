---
id: CES-74
title: The logger module
references: ["structured-logging"]
---

## Directive

Logging is configured in exactly one place: a `core/logger.py` module built on
[structlog](https://www.structlog.org). Its code is the topic `structured-logging`
(`botica refs show structured-logging`). Copy it to `<your_package>/core/logger.py` and import
`get_logger` everywhere. This is the structure that `log-get-logger` (CES-45) and `log-no-print`
(CES-46) point at.

## Why

Events are key/value pairs and not formatted strings, so they stay queryable in a log sink. One
place of configuration keeps the format, the level and the destination the same in every module,
and gives CES-45 and CES-46 a single way to get a logger.
