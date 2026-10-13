"""CES-76 · settings-module — the one place env/flags are read (pydantic-settings).

Drop-in / comparison template for ``<pkg>/settings.py`` (or a ``settings/`` package). Per-module
application config lives in a ``pydantic_settings.BaseSettings`` subclass that:

- reads env **case-insensitively** (``DEBUG`` / ``debug`` both bind ``debug``);
- supports a ``.env`` file and an env prefix;
- is reached everywhere else through a cached ``get_settings()`` — never a scattered
  ``os.getenv`` / ``os.environ`` (which ``settings-module`` flags outside this module).

See ``uvx botica explain settings-module`` for the full rule.
"""

from __future__ import annotations

from functools import lru_cache

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_prefix="APP_",
        env_file=".env",
        case_sensitive=False,  # DEBUG and debug both bind `debug`
        extra="ignore",
    )

    debug: bool = False
    log_level: str = "INFO"
    database_url: str = Field(default="sqlite:///./app.db")


@lru_cache
def get_settings() -> Settings:
    """Return the process-wide settings, constructed (and validated) once."""
    return Settings()


# Anti-pattern (flagged by CES-76 outside this module) — do NOT read env ad hoc:
#
#     import os
#     debug = os.getenv("APP_DEBUG")   # settings-module
