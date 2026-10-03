# CES-74 · the house logger (core/logger.py)

**Code:** `CES-74` &nbsp;·&nbsp; **Slug:** `core-logger` &nbsp;·&nbsp; **Ships as:** snippet
(`.agents/snippets/core/logger.py`) &nbsp;·&nbsp; **Tracker:**
[#74](https://github.com/collectiveai-team/botica/issues/74)

## Directive

Logging is configured in exactly one place: a `core/logger.py` module built on
[structlog](https://www.structlog.org), shipped as the drop-in `.agents/snippets/core/logger.py`. Copy
it to `<your_package>/core/logger.py` and import `get_logger` everywhere. This is the structure
that `log-get-logger` (CES-45) and `log-no-print` (CES-46) point at.

This is a **snippet**, not auto-generated runtime code — the scaffolder never writes into your
package path. You drop it in and wire it up.

## Behaviour

| Environment | Rendering |
|---|---|
| `ENV=prod` / `ENV=production` | structured **JSON** to stdout (machine-parseable) |
| anything else (dev) | **colored console** rendering (human-friendly) |

The level is read once from `LOG_LEVEL` (default `INFO`, case-insensitive). The stack is
configured lazily on the first `get_logger` call, so importing the module is free.

## Usage

```python
from myapp.core.logger import get_logger

log = get_logger(__name__)
log = log.bind(request_id=rid)          # bind context once
log.info("user_created", user_id=u.id) # emit events as key/value pairs
```

## Why structured logging

- Events are key/value pairs, not pre-formatted strings — they stay queryable in a log sink.
- One config point keeps every module consistent (format, level, destination) and makes
  CES-45 / CES-46 enforceable: there is a single right way to get a logger.

## Adapting

Change the package import path to match your layout; keep the JSON/console split and the
`LOG_LEVEL` contract so the behaviour matches what the standards assume.
