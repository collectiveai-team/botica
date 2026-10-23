"""The access tokens that the service signs, with PyJWT, and the tokens of an identity provider."""

from __future__ import annotations

from datetime import timedelta
from functools import lru_cache
from typing import TYPE_CHECKING

import jwt

if TYPE_CHECKING:
    from collections.abc import Callable
    from datetime import datetime

    from app.impl.settings import AuthSettings

ALGORITHM = "HS256"


class InvalidTokenError(Exception):
    """The token is malformed, signed with another key, expired or meant for someone else."""


def issue_access_token(user_id: str, settings: AuthSettings, now: datetime) -> str:
    """Sign an access token for the user that expires after the lifetime of the settings."""
    claims = {
        "sub": user_id,
        "iss": settings.issuer,
        "aud": settings.audience,
        "iat": now,
        "exp": now + timedelta(seconds=settings.access_token_ttl_seconds),
    }
    return jwt.encode(claims, settings.signing_key.get_secret_value(), algorithm=ALGORITHM)


def verify_access_token(token: str, settings: AuthSettings) -> str:
    """Return the user of an access token of the service, after checking all of it."""
    try:
        claims = jwt.decode(
            token,
            settings.signing_key.get_secret_value(),
            algorithms=[ALGORITHM],
            audience=settings.audience,
            issuer=settings.issuer,
            options={"require": ["exp", "iat", "sub"]},
        )
    except jwt.PyJWTError as error:
        raise InvalidTokenError from error
    return claims["sub"]


@lru_cache
def provider_keys(jwks_url: str) -> jwt.PyJWKClient:
    """Return the client of the keys of the provider: one per process, with its cache of keys."""
    return jwt.PyJWKClient(jwks_url)


def verify_provider_token(
    token: str, settings: AuthSettings, signing_key_of: Callable[[str], jwt.PyJWK]
) -> str:
    """Return the user of a token of the identity provider, after checking all of it.

    `signing_key_of` finds the key of the token among the keys the provider publishes, such as
    `provider_keys(settings.provider_jwks_url).get_signing_key_from_jwt`.
    """
    try:
        key = signing_key_of(token)
        claims = jwt.decode(
            token,
            key,
            algorithms=["RS256", "ES256"],
            audience=settings.provider_audience,
            issuer=settings.provider_issuer,
            options={"require": ["exp", "iat", "sub"]},
        )
    except jwt.PyJWTError as error:
        raise InvalidTokenError from error
    return claims["sub"]
