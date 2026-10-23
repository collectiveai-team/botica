"""The settings of the database package."""

from __future__ import annotations

from functools import lru_cache

from pydantic_settings import BaseSettings, SettingsConfigDict


class DatabaseSettings(BaseSettings):
    """The connection to the database, read with the prefix `APP_DATABASE_`."""

    model_config = SettingsConfigDict(
        env_prefix="APP_DATABASE_",
        env_file=".env",
        case_sensitive=False,
        extra="ignore",
    )

    url: str = "sqlite:///./app.db"
    echo: bool = False


@lru_cache
def get_settings() -> DatabaseSettings:
    """Return the settings of the database, built and validated once."""
    return DatabaseSettings()
