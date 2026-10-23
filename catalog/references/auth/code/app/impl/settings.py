"""The settings of authentication: the key, the issuer, the audience and the lifetimes."""

from __future__ import annotations

from functools import lru_cache

from pydantic import Field, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict


class AuthSettings(BaseSettings):
    """The variables of authentication, read with the prefix `APP_AUTH_`.

    A lifetime is a number of seconds, such as `APP_AUTH_ACCESS_TOKEN_TTL_SECONDS=900`.
    """

    model_config = SettingsConfigDict(
        env_prefix="APP_AUTH_",
        env_file=".env",
        case_sensitive=False,
        extra="ignore",
    )

    signing_key: SecretStr = Field(min_length=32)
    issuer: str
    audience: str
    access_token_ttl_seconds: int = Field(default=900, gt=0)
    refresh_token_ttl_seconds: int = Field(default=14 * 24 * 3600, gt=0)
    provider_jwks_url: str = ""
    provider_issuer: str = ""
    provider_audience: str = ""


@lru_cache
def get_settings() -> AuthSettings:
    """Return the settings of authentication, built and validated once."""
    return AuthSettings()
