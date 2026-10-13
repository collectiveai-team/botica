---
title: Define flows as async @flow and each unit of work as a @task
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, flows, tasks, async, orchestration
---

## Define flows as async @flow and each unit of work as a @task

Flows are `async def` functions decorated with `@flow`; every discrete unit of work
is an `async def` decorated with `@task`. This boundary lets Prefect track retries,
emit state events, and persist results independently per task. Always set
`persist_result=True` and `log_prints=True` on the `@flow` so output is visible in
the Prefect UI without extra wiring. Use `get_run_logger()` inside flows and tasks
rather than a bare `print` or module-level logger.

**Prefer:**

```python
from prefect import flow, task, get_run_logger

@flow(name="my_flow", persist_result=True, log_prints=True)
async def my_flow(document_id: str) -> MyResult:
    logger = get_run_logger()
    logger.info("starting", document_id=document_id)
    text = await extract_text(document_id)
    return await classify(text)

@task(name="extract_text")
async def extract_text(document_id: str) -> str:
    ...
```

**Avoid:**

```python
# bare async function — Prefect can't track it, retry it, or persist its result
async def extract_text(document_id: str) -> str:
    print("extracting")   # use get_run_logger() instead
    ...
```

Reference: see `prefect-parallel-tasks` for running independent tasks concurrently
and `prefect-artifacts` for making task I/O inspectable in the UI.
