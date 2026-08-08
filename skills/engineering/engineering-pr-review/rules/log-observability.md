---
title: Add Logfire instrumentation in production services; use get_run_logger in Prefect
section: log
scope: specific
applies-to: fastapi, prefect
status: current
tags: logging, logfire, prefect, observability, production
---

## Add Logfire instrumentation in production services; use get_run_logger in Prefect

Production FastAPI services layer Logfire on top of `get_logger`. Activate it
conditionally on `LOGFIRE_TOKEN` so the service runs without the token in dev/test.
Instrument FastAPI, Pydantic AI, httpx, and Pydantic at startup. Inside Prefect
flows and tasks, use `get_run_logger()` — it routes to the Prefect UI run log rather
than the root Python logger. Langfuse is for notebook/experiment tracing only, not
production services.

**Prefer (FastAPI service startup):**

```python
# api/main.py
import logfire
from myapp.core.settings import settings

if settings.LOGFIRE_TOKEN:
    logfire.configure(
        token=settings.LOGFIRE_TOKEN.get_secret_value(),
        service_name=settings.APP_NAME,
        service_version=settings.APP_VERSION,
        environment=settings.STAGE,
    )

logfire.instrument_pydantic()
logfire.instrument_pydantic_ai()
logfire.instrument_httpx(capture_request_body=True, capture_response_body=True)

# after app = FastAPI(...)
logfire.instrument_fastapi(app, excluded_urls=r"^https?://[^/]+/basics/health$")
```

**Prefer (Prefect flows/tasks):**

```python
from prefect import flow, get_run_logger

@flow
async def my_flow(doc_id: str) -> None:
    logger = get_run_logger()
    logger.info(f"Starting flow for doc {doc_id}")
```

**Avoid:**

```python
# Hardcoding the token — use LOGFIRE_TOKEN env var so it's absent in tests
logfire.configure(token="lf_abc123...")

# Using root logger inside a Prefect task — output goes to console, not Prefect UI
import logging
logger = logging.getLogger(__name__)
```

Reference: `log-get-logger-rich` for the base logging factory. Langfuse is appropriate
for experiment and notebook tracing; Logfire is for production service observability.
