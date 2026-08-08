---
title: Run independent tasks concurrently with asyncio.gather inside a flow
section: prefect
scope: specific
applies-to: prefect
status: current
tags: prefect, async, concurrency, gather, performance
---

## Run independent tasks concurrently with asyncio.gather inside a flow

When two or more `@task` calls share no data dependency within a flow, wrap them
in a single `asyncio.gather` call so they execute concurrently. This is the standard
Prefect 3.x pattern for fan-out within a flow — no additional executor config
required. Only gather tasks that are truly independent; tasks that feed into each
other must remain sequential.

**Prefer:**

```python
import asyncio

# extract_parties, extract_clauses, extract_dates share only the input text
parties, clauses, dates = await asyncio.gather(
    extract_parties(text=extracted_text, artifact_key="parties"),
    extract_clauses(text=extracted_text, artifact_key="clauses"),
    extract_dates(text=extracted_text, artifact_key="dates"),
)
```

**Avoid:**

```python
# sequential calls when there's no dependency — wastes wall-clock time
parties = await extract_parties(text=extracted_text)
clauses = await extract_clauses(text=extracted_text)
dates   = await extract_dates(text=extracted_text)
```

A typical pattern: sequential steps where each feeds the next (`extract_document` →
`classify_document`), then `asyncio.gather` fans out to independent extraction tasks
before a final sequential step that depends on the gathered output.

Reference: see `prefect-flows-tasks` for the `@flow`/`@task` skeleton.
