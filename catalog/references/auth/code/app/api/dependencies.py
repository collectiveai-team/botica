"""The dependencies that give a route its caller, its clock and the sessions."""

from __future__ import annotations

from datetime import UTC, datetime
from typing import Annotated

from fastapi import Cookie, Depends, HTTPException, status
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from sqlmodel import Session

from app.core.sessions import Sessions
from app.database.repositories.refresh_tokens import SqlRefreshTokenStore
from app.database.services.session import get_session
from app.impl.access_tokens import InvalidTokenError, verify_access_token
from app.impl.settings import AuthSettings, get_settings

ACCESS_COOKIE = "__Host-access"

bearer = HTTPBearer(auto_error=False)

DatabaseSession = Annotated[Session, Depends(get_session)]


def get_now() -> datetime:
    """Return the time of the request; a test replaces it to move the clock."""
    return datetime.now(UTC)


def get_sessions(
    session: DatabaseSession, settings: Annotated[AuthSettings, Depends(get_settings)]
) -> Sessions:
    """Return the sessions over the table of refresh tokens, shared by every process."""
    return Sessions(SqlRefreshTokenStore(session), settings.refresh_token_ttl_seconds)


def get_current_user(
    credentials: Annotated[HTTPAuthorizationCredentials | None, Depends(bearer)],
    settings: Annotated[AuthSettings, Depends(get_settings)],
    access_cookie: Annotated[str | None, Cookie(alias=ACCESS_COOKIE)] = None,
) -> str:
    """Return the id of the caller from its access token, or answer 401.

    The token comes in the header `Authorization: Bearer`, or in the cookie of a browser session.
    """
    token = credentials.credentials if credentials is not None else access_cookie
    if token is None:
        raise _unauthorized()
    try:
        return verify_access_token(token, settings)
    except InvalidTokenError as error:
        raise _unauthorized() from error


def _unauthorized() -> HTTPException:
    return HTTPException(
        status.HTTP_401_UNAUTHORIZED, "invalid or expired token", {"WWW-Authenticate": "Bearer"}
    )


CurrentUser = Annotated[str, Depends(get_current_user)]
Now = Annotated[datetime, Depends(get_now)]
