---
title: Store prompts as versioned .prompt.yml files co-located with tasks
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, prompts, yaml, langfuse, versioning
---

## Store prompts as versioned .prompt.yml files co-located with tasks

Prompts are stored as `.prompt.yml` files next to the task code that uses them.
Each file carries `version` and `fetched_at` metadata so the exact prompt used in
production is auditable in git. Load prompts from files via a helper — do not
inline multi-line prompts in Python code.

**Prefer:**

```python
# helpers/prompt_loader.py
from functools import lru_cache
import yaml
from pathlib import Path

TASKS_DIR = Path(__file__).parent.parent / "tasks"

@lru_cache(maxsize=None)
def _load_yaml(filename: str) -> dict:
    return yaml.safe_load((TASKS_DIR / filename).read_text())

def load_system_prompt(filename: str) -> str:
    return _load_yaml(filename)["prompt"]

# In the task:
system_prompt = load_system_prompt("clauses_extraction.prompt.yml")
```

**Prompt file format:**

```yaml
# tasks/clauses_extraction.prompt.yml
version: 2
fetched_at: '2025-12-06T01:03:19.890514+00:00'
prompt: |
  You are a contract analyst. Extract all clauses ...
```

**Avoid:**

```python
# Hardcoded prompt strings in task code — not versioned, not auditable
system_prompt = """You are a contract analyst..."""
```

When using Langfuse for prompt management, snapshot prompts to `.prompt.yml` files
at version time rather than fetching them at runtime — this keeps production
independent of the Langfuse service.

Reference: `llm-observability` for Langfuse's role (experiments/prompt management
only, not wired into prod).
