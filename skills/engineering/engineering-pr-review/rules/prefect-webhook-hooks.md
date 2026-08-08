---
title: Report flow state changes via webhook lifecycle hooks
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, webhooks, hooks, lifecycle, notifications
---

## Report flow state changes via webhook lifecycle hooks

Flow state changes (running, completion, failure, crash, cancellation) are reported
to the caller by registering the same webhook function on all five `@flow` lifecycle
hook parameters. The function POSTs a Prefect `Event` payload to a `webhook_url`
passed as a flow parameter, with exponential backoff (`MAX_RETRIES=5`,
`BACKOFF_FACTOR=2`).

**This is a deliberate workaround.** Prefect 3.4.x automations did not behave as
expected for this use-case. When Prefect automations become reliable, replace these
hooks with a native automation that POSTs to the same endpoint.

**Prefer:**

```python
from workflows.hooks.webhook import webhook_event_flowrun

@flow(
    name="my_flow",
    on_completion=[webhook_event_flowrun],
    on_failure=[webhook_event_flowrun],
    on_crashed=[webhook_event_flowrun],
    on_running=[webhook_event_flowrun],
    on_cancellation=[webhook_event_flowrun],
)
async def my_flow(document_id: str, webhook_url: str | None = None) -> MyResult:
    ...
# webhook_url is read from flow_run.parameters inside the hook
```

**Avoid:**

```python
# registering only on_completion — callers won't know about failures or crashes
@flow(on_completion=[notify])
async def my_flow(...): ...
```

The hook builds a `prefect.events.schemas.events.Event` and POSTs it via `httpx` with
a `time.sleep(BACKOFF_FACTOR**retry)` loop up to `WEBHOOK_MAX_RETRIES=5`. Suggested
config constants: `WEBHOOK_MAX_RETRIES=5`, `WEBHOOK_TIMEOUT=10`, `WEBHOOK_BACKOFF_FACTOR=2`.

Reference: see `prefect-flows-tasks` for the `@flow` skeleton and `prefect-api-bridge`
for how the API side triggers and polls deployments.
