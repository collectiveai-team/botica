---
title: Pydantic v2 for domain data
section: pylayout
scope: general
applies-to: all
status: current
tags: pydantic, domain-models, validation
---

## Pydantic v2 for domain data

Use Pydantic v2 for domain data shapes (request/response models, retrieved documents,
structured LLM output). How an app reads its configuration is a separate matter:
see `api-pydantic-settings`.

**Prefer (domain data):**

```python
# PEP 604 unions (v2 native), model_dump_json, Field with default_factory
from pydantic import BaseModel, Field

class RetrievedDocument(BaseModel):
    content: str
    score: float
    metadata: dict[str, str] = Field(default_factory=dict)

doc.model_dump_json()        # v2 — not .json() (deprecated)
```

**Avoid:**

```python
doc.dict()      # v1 API — deprecated in v2
doc.json()      # v1 API — use model_dump_json() instead
Optional[int]   # use int | None instead (v2 style)
```

Libraries use Pydantic for data shapes only.
