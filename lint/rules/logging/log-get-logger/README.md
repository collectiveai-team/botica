# log-get-logger · `BTLOG001`

**Standard:** CES-45 (documentation only) · **Family:** logging · **Languages:** Python

## Directive

Never call `logging.getLogger(...)`, or a bare `getLogger(...)`, directly. Get loggers from the
house `get_logger` of `core/logger.py`, the structlog setup shipped as
`.agents/snippets/core/logger.py`. A warning by default.

## Why

- One configuration point makes every module log the same way: JSON in production, a coloured
  console in development, the level from `LOG_LEVEL`. Scattered `getLogger` calls drift into
  inconsistent formats and lost context.
- `get_logger` returns a structlog `BoundLogger`, so you can `.bind(request_id=...)` context and
  emit key/value events instead of pre-formatted strings.

## Do

```python
from myapp.core.logger import get_logger

log = get_logger(__name__)
log.info("user_created", user_id=user.id)
```

## Don't

```python
import logging

log = logging.getLogger(__name__)
```

## Suppression

Silence one line with `# botica: ignore[log-get-logger]` or `# noqa: BTLOG001`.
