# settings-module · `BTARCH002`

**Standard:** CES-76 (documentation only) · **Family:** architecture · **Languages:** Python

## Directive

Read the environment in one place: a `pydantic_settings.BaseSettings` subclass in the module's
`settings.py` (or `settings/` package), reached everywhere else through a cached
`get_settings()`. `os.getenv(...)` and `os.environ` anywhere else are reported. A warning by
default.

## Why

- Scattered `os.getenv` calls are an invisible, untyped and undocumented configuration surface:
  no defaults in one place, no validation, no list of what the application reads.
- A `BaseSettings` class gives typed fields, defaults, `.env` support, validation at start-up and
  one inventory of every knob.

## What is exempt

The settings module itself, since it is where the environment is read: `settings.py`, anything
under `settings/`, `config.py`, anything under `config/`, `conftest.py` and anything under
`tests/`.

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

Silence one line with `# botica: ignore[settings-module]` or `# noqa: BTARCH002`. The rule is
written in ast-grep's YAML (`python.yml`), so `# ast-grep-ignore: settings-module` also works while
`compat-ast-grep-ignore` is on.
