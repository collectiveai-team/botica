# settings-base-class · `BTARCH003`

**Standard:** none yet · **Family:** architecture · **Languages:** Python · **Status:** preview, off by default (set `preview = true` and select it)

## Directive

A class whose name ends with `Settings` extends `pydantic_settings.BaseSettings`, not
`BaseModel`. A warning by default.

## Why

A `BaseModel` named `Settings` reads nothing from the environment: the values are filled in by
hand, usually with the `os.environ` reads that the standard keeps in one place.

## Do

```python
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    database_url: str
```

## Don't

```python
from pydantic import BaseModel


class Settings(BaseModel):
    database_url: str
```

## Suppression

Silence one line with `# botica: ignore[settings-base-class]` or `# noqa: BTARCH003`.
