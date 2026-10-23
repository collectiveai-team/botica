"""The session of a client that holds its access token: sign in, refresh and sign out.

The access token goes back in the body, and the client sends it as `Authorization: Bearer`. The
refresh token travels in a cookie.
"""

from collections.abc import Callable
from typing import TYPE_CHECKING, Annotated

import jwt
from fastapi import APIRouter, Cookie, Depends, HTTPException, Response, status

from app.api.dependencies import Now, get_sessions
from app.api.v1.schemas.requests.auth import SignInRequest
from app.api.v1.schemas.responses.auth import AccessTokenResponse
from app.core.sessions import InvalidRefreshTokenError, Sessions
from app.impl.access_tokens import (
    InvalidTokenError,
    issue_access_token,
    provider_keys,
    verify_provider_token,
)
from app.impl.settings import AuthSettings, get_settings

if TYPE_CHECKING:
    from datetime import datetime

router = APIRouter(prefix="/auth", tags=["auth"])

REFRESH_COOKIE = "refresh_token"
REFRESH_PATH = "/v1/auth"

Settings = Annotated[AuthSettings, Depends(get_settings)]
SessionsDep = Annotated[Sessions, Depends(get_sessions)]


def get_signing_key_of(settings: Settings) -> Callable[[str], jwt.PyJWK]:
    """Return the function that finds the key of a token among the keys of the provider."""
    return provider_keys(settings.provider_jwks_url).get_signing_key_from_jwt


SigningKeyOf = Annotated[Callable[[str], jwt.PyJWK], Depends(get_signing_key_of)]


def user_of_provider_token(
    id_token: str, settings: AuthSettings, signing_key_of: Callable[[str], jwt.PyJWK]
) -> str:
    """Return the user of a token of the identity provider, or answer 401."""
    try:
        return verify_provider_token(id_token, settings, signing_key_of)
    except InvalidTokenError as error:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "invalid token") from error


@router.post("/session")
def sign_in(
    payload: SignInRequest,
    response: Response,
    settings: Settings,
    sessions: SessionsDep,
    now: Now,
    signing_key_of: SigningKeyOf,
) -> AccessTokenResponse:
    """Open a session for the user of a token of the identity provider."""
    user_id = user_of_provider_token(payload.id_token, settings, signing_key_of)
    _set_refresh_cookie(response, sessions.open(user_id, now), settings)
    return _access_token(user_id, settings, now)


@router.post("/refresh")
def refresh(
    response: Response,
    settings: Settings,
    sessions: SessionsDep,
    now: Now,
    refresh_token: Annotated[str | None, Cookie(alias=REFRESH_COOKIE)] = None,
) -> AccessTokenResponse:
    """Replace the refresh token of the cookie and return a new access token."""
    try:
        user_id, replacement = sessions.rotate(refresh_token or "", now)
    except InvalidRefreshTokenError as error:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "sign in again") from error
    _set_refresh_cookie(response, replacement, settings)
    return _access_token(user_id, settings, now)


@router.post("/logout", status_code=status.HTTP_204_NO_CONTENT)
def sign_out(
    response: Response,
    sessions: SessionsDep,
    refresh_token: Annotated[str | None, Cookie(alias=REFRESH_COOKIE)] = None,
) -> None:
    """End the session of the refresh token of the cookie."""
    if refresh_token is not None:
        sessions.close(refresh_token)
    response.delete_cookie(REFRESH_COOKIE, path=REFRESH_PATH)


def _access_token(user_id: str, settings: AuthSettings, now: datetime) -> AccessTokenResponse:
    return AccessTokenResponse(
        access_token=issue_access_token(user_id, settings, now),
        expires_in=settings.access_token_ttl_seconds,
    )


def _set_refresh_cookie(response: Response, token: str, settings: AuthSettings) -> None:
    response.set_cookie(
        REFRESH_COOKIE,
        token,
        max_age=settings.refresh_token_ttl_seconds,
        path=REFRESH_PATH,
        secure=True,
        httponly=True,
        samesite="strict",
    )
