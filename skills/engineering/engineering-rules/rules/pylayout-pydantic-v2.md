---
title: Pydantic v2 for domain data; pydantic-settings for app config
section: pylayout
scope: general
applies-to: all
status: current
tags: pydantic, pydantic-settings, domain-models, config, validation
---

## Pydantic v2 for domain data; pydantic-settings for app config

Use Pydantic v2 for domain data shapes (request/response models, retrieved documents,
structured LLM output). Use `pydantic-settings` (`BaseSettings`) for reading app
configuration from environment variables. Libraries with no app config surface read env
directly rather than bundling a settings object.

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

**Prefer (app config — services only):**

```python
from pydantic_settings import BaseSettings

class Settings(BaseSettings):
    database_url: str
    log_level: str = "INFO"
```

**Avoid:**

```python
from pydantic import BaseModel

class Settings(BaseModel):   # use BaseSettings, not BaseModel, for env config
    database_url: str = os.environ["DATABASE_URL"]

doc.dict()      # v1 API — deprecated in v2
doc.json()      # v1 API — use model_dump_json() instead
Optional[int]   # use int | None instead (v2 style)
```

Libraries use Pydantic for data shapes only — no settings class needed unless the library
exposes a service or CLI.
