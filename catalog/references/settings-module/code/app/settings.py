"""The settings of the package: the one module that reads the environment and the flags."""

from __future__ import annotations

from functools import lru_cache

from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """The variables of the package, read with the prefix `APP_` from the environment and `.env`."""

    model_config = SettingsConfigDict(
        env_prefix="APP_",
        env_file=".env",
        case_sensitive=False,
        extra="ignore",
    )

    debug: bool = False
    log_level: str = "INFO"
    database_url: str = Field(default="sqlite:///./app.db")


@lru_cache
def get_settings() -> Settings:
    """Return the settings of the process, built and validated once."""
    return Settings()
