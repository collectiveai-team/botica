# CES-76 · config lives in a settings module

**Code:** `CES-76` &nbsp;·&nbsp; **Slug:** `settings-module` &nbsp;·&nbsp; **Enforced by:** ast-grep
(`warning`) &nbsp;·&nbsp; **Tracker:**
[#76](https://github.com/collectiveai-team/botica/issues/76)

## Directive

Application config is read in exactly one place: a `pydantic_settings.BaseSettings` subclass in
the module's `settings.py` (or `settings/` package), reached everywhere else via a cached
`get_settings()`. Reading env directly — `os.getenv(...)`, `os.environ[...]`,
`os.environ.get(...)` — anywhere outside the settings module is flagged.

## Why

- Scattered `os.getenv` calls are an invisible, untyped, undocumented config surface: no
  defaults in one place, no validation, no list of what the app actually reads.
- A `BaseSettings` class gives you typed fields, defaults, `.env` support, validation at
  startup, and one greppable inventory of every knob. `get_settings()` makes access cached and
  testable (override the dependency instead of monkeypatching the environment).

## Rules

- Settings are **case-insensitive** (`DEBUG` and `debug` bind the same field).
- Access is through `get_settings()`, never by constructing `Settings()` ad hoc at call sites.
- The settings module itself is exempt — it is *supposed* to read env. So are `tests/` and
  `conftest.py`.

## Do

```python
from myapp.settings import get_settings

settings = get_settings()
if settings.debug:
    ...
```

See `.agents/snippets/settings.py` for the canonical `BaseSettings` + `get_settings()` module.

## Don't

```python
import os

if os.getenv("APP_DEBUG"):   # settings-module
    ...
```

## Suppressing (rare, must be visible)

A genuine one-off (e.g. reading `PATH` in a build script) keeps the call and adds a visible
ignore:

```python
venv = os.environ.get("VIRTUAL_ENV")  # ast-grep-ignore: settings-module
```

The slug is the stable suppression key; the `CES-76` code in the message is for humans.
