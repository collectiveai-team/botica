# CES-45 · use the house get_logger

**Code:** `CES-45` &nbsp;·&nbsp; **Slug:** `log-get-logger` &nbsp;·&nbsp; **Enforced by:** ast-grep
(`warning`) &nbsp;·&nbsp; **Tracker:**
[#45](https://github.com/collectiveai-team/botica/issues/45)

## Directive

Never call `logging.getLogger(...)` (or a bare `getLogger(...)`) directly. Acquire loggers
through the house `get_logger` from `core/logger.py` — the canonical structlog setup shipped as
`.agents/snippets/core/logger.py` (CES-74).

## Why

- One configuration point means every module logs the same way: JSON in prod, colored console
  in dev, level from `LOG_LEVEL`. Scattered `getLogger` calls drift into inconsistent formats
  and lost structured context.
- `get_logger` returns a structlog `BoundLogger`, so you can `.bind(request_id=…)` context and
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

log = logging.getLogger(__name__)  # log-get-logger
```

The drop-in lives at `.agents/snippets/core/logger.py` — copy it to `<your_package>/core/logger.py`.

## Suppressing (rare, must be visible)

If you must use the stdlib logger directly (e.g. configuring a third-party library's logger),
keep the call and add a visible suppression so the exception is auditable:

```python
logging.getLogger("uvicorn.access").setLevel("WARNING")  # ast-grep-ignore: log-get-logger
```

The slug is the stable suppression key; the `CES-45` code in the message is for humans.
