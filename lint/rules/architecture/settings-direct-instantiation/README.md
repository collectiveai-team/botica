## Directive

A settings class is built in one place, the cached `get_settings()`. A call such as
`Settings()` or `AppSettings(...)` anywhere else is reported. A warning by default.

## Why

Every call re-reads the environment, and a test cannot override the settings of a module that
built its own.

## Do

```python
from myapp.settings import get_settings

settings = get_settings()
```

## Don't

```python
settings = Settings()
```

## Suppression

Silence one line with `# botica: ignore[settings-direct-instantiation]` or `# noqa: BTARCH004`.
