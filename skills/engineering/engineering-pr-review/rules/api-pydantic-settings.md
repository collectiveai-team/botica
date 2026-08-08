---
title: Expose app config as a cached singleton via get_settings()
section: api
scope: specific
applies-to: fastapi
status: current
tags: fastapi, config, pydantic-settings, settings, lru_cache
---

## Expose app config as a cached singleton via get_settings()

App configuration is defined in a `pydantic-settings` `BaseSettings` subclass and exposed
through a single `@lru_cache`-decorated factory. Callers always call `get_settings()` —
they never instantiate `Settings()` directly. The cache ensures only one `Settings` object
exists per process, env vars are read once, and FastAPI's `Depends(get_settings)` works
without rebuilding the object on every request.

**Prefer:**

```python
# app/config.py
from functools import lru_cache
from pydantic_settings import BaseSettings, SettingsConfigDict

class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_file=(".env", ".env.dev"), extra="ignore")

    database_url: str = "sqlite:///./app.db"
    api_prefix: str = "/api"
    disable_auth: bool = False

@lru_cache
def get_settings() -> Settings:
    return Settings()

# anywhere else
from app.config import get_settings
settings = get_settings()   # ✓ cached singleton
```

**Avoid:**

```python
settings = Settings()   # ✗ re-reads env on every import; breaks test overrides
```

**Direction:** prefer `@lru_cache` + `get_settings()` over a bare module-level
`settings = Settings()` — the function form allows `app.dependency_overrides` in tests.
