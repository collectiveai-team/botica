---
id: CES-76
title: One settings module per package
references: ["settings-module"]
---

## Directive

1. Each layer or package has its own `settings.py`.
2. It defines a `pydantic_settings.BaseSettings` subclass that is case-insensitive
   (`SettingsConfigDict(case_sensitive=False)`).
3. It exposes a cached `get_settings()` and nobody builds the settings class directly.
4. `os.getenv` and `os.environ` are used only inside a settings module.
5. A higher layer imports the settings of the layers below and stacks them as fields of its own.
6. In a project with a varlock schema (`.env.schema`), the schema declares each variable that a
   settings class reads, and a settings class reads each variable of the schema. A variable that
   another program reads carries `# @auditIgnore` in the schema.

```python
@lru_cache
def get_settings() -> DatabaseSettings:
    return DatabaseSettings()
```

## Why

Configuration enters the process once, through a typed and validated object, and the settings
class of a package lists what the package needs. The schema lists what a deployment sets and
validates; when the two lists differ, a variable is read with no validation or set for nothing.
