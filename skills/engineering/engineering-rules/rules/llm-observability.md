---
title: Logfire for production tracing; Langfuse for experiments and notebooks only
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, observability, logfire, langfuse, tracing
---

## Logfire for production tracing; Langfuse for experiments and notebooks only

Production services instrument with **Logfire** (pydantic-ai's native OTel
backend). **Langfuse** is used only in notebooks and prompt-engineering
experiments — it is not wired into any production code path. Keep this
separation: Logfire in prod, Langfuse in notebooks.

**Prefer (production):**

```python
# app/main.py — app startup
import logfire

logfire.configure(...)
logfire.instrument_pydantic()
logfire.instrument_pydantic_ai()   # traces every Agent run
logfire.instrument_fastapi(app)
```

Pass `instrument=True` when constructing agents to enable per-run tracing
picked up by `Agent.instrument_all()`.

**Avoid (in production code):**

```python
# Do not import or configure Langfuse in production app or workflow paths
from langfuse import Langfuse
from langfuse.decorators import observe
```

Langfuse belongs in notebooks for dataset creation, prompt iteration, and
manual tracing — not in the production module graph.

Reference: `llm-prompts-yaml` for Langfuse's other role (prompt snapshot source).
