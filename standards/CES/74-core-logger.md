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

Botica never writes into your package path: you copy the code and wire it up.

## Behaviour

| Environment | Rendering |
|---|---|
| `ENV=prod` or `ENV=production` | structured JSON to stdout (machine-parseable) |
| anything else (dev) | colored console rendering (human-friendly) |

The level is read once from `LOG_LEVEL` (default `INFO`, case-insensitive). The stack is configured
lazily on the first `get_logger` call, so importing the module configures nothing.

## Usage

```python
from myapp.core.logger import get_logger

log = get_logger(__name__)
log = log.bind(request_id=rid)  # bind context once
log.info("user_created", user_id=u.id)  # emit events as key/value pairs
```

## Why structured logging

- Events are key/value pairs and not pre-formatted strings, so they stay queryable in a log sink.
- One config point keeps every module consistent (format, level, destination) and makes CES-45 and
  CES-46 enforceable: there is a single way to get a logger.

## Adapting

Change the package import path to match your layout. Keep the JSON and console split and the
`LOG_LEVEL` variable, which the standards of logging assume.
