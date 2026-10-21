## Directive

Read the environment in one place: the settings module. `os.getenv(...)` and `os.environ`
anywhere else are reported. A warning by default.

## Why

Scattered `os.getenv` calls leave the configuration with no type and no documentation. Its
defaults are spread across files, nothing validates it, and nothing lists the variables the
application reads.

## Exceptions

The rule skips the files where the environment is read on purpose:

- `settings.py` and every file under `settings/`.
- `conftest.py` and every file under `tests/`.

## Do

```python
from myapp.settings import get_settings

settings = get_settings()
if settings.debug:
    ...
```

## Don't

```python
import os

if os.getenv("APP_DEBUG"):
    ...
```

## Suppression

A comment on the line of the `os.getenv(...)` or `os.environ` read silences it.

```python
url = os.getenv("DATABASE_URL")  # botica: ignore[settings-module]
url = os.getenv("DATABASE_URL")  # noqa: BTCONF001 -- reason
```
