"""The session of a browser, with both tokens in `HttpOnly` cookies: sign in, refresh, sign out.

No script of the page reads either token. The browser sends the access cookie to every route of
the origin, and the refresh cookie only to the routes of this router.
"""

from typing import TYPE_CHECKING, Annotated

from fastapi import APIRouter, Cookie, HTTPException, Response, status

from app.api.dependencies import ACCESS_COOKIE, Now
from app.api.v1.routers.auth import SessionsDep, Settings, SigningKeyOf, user_of_provider_token
from app.api.v1.schemas.requests.auth import SignInRequest
from app.core.sessions import InvalidRefreshTokenError
from app.impl.access_tokens import issue_access_token

if TYPE_CHECKING:
    from datetime import datetime

    from app.impl.settings import AuthSettings

router = APIRouter(prefix="/browser", tags=["auth"])

REFRESH_COOKIE = "__Secure-refresh"
REFRESH_PATH = "/v1/browser"


@router.post("/session", status_code=status.HTTP_204_NO_CONTENT)
def browser_sign_in(
    payload: SignInRequest,
    response: Response,
    settings: Settings,
    sessions: SessionsDep,
    now: Now,
    signing_key_of: SigningKeyOf,
) -> None:
    """Open a session for the user of a token of the identity provider, in two cookies."""
    user_id = user_of_provider_token(payload.id_token, settings, signing_key_of)
    _set_cookies(response, user_id, sessions.open(user_id, now), settings, now)


@router.post("/refresh", status_code=status.HTTP_204_NO_CONTENT)
def browser_refresh(
    response: Response,
    settings: Settings,
    sessions: SessionsDep,
    now: Now,
    refresh_token: Annotated[str | None, Cookie(alias=REFRESH_COOKIE)] = None,
) -> None:
    """Replace the refresh token of the cookie, and the access token with it."""
    try:
        user_id, replacement = sessions.rotate(refresh_token or "", now)
    except InvalidRefreshTokenError as error:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "sign in again") from error
    _set_cookies(response, user_id, replacement, settings, now)


@router.post("/logout", status_code=status.HTTP_204_NO_CONTENT)
def browser_sign_out(
    response: Response,
    sessions: SessionsDep,
    refresh_token: Annotated[str | None, Cookie(alias=REFRESH_COOKIE)] = None,
) -> None:
    """End the session of the refresh token of the cookie, and delete both cookies."""
    if refresh_token is not None:
        sessions.close(refresh_token)
    response.delete_cookie(ACCESS_COOKIE, path="/", secure=True, httponly=True, samesite="strict")
    response.delete_cookie(
        REFRESH_COOKIE, path=REFRESH_PATH, secure=True, httponly=True, samesite="strict"
    )


def _set_cookies(
    response: Response, user_id: str, refresh_token: str, settings: AuthSettings, now: datetime
) -> None:
    response.set_cookie(
        ACCESS_COOKIE,
        issue_access_token(user_id, settings, now),
        max_age=settings.access_token_ttl_seconds,
        path="/",
        secure=True,
        httponly=True,
        samesite="strict",
    )
    response.set_cookie(
        REFRESH_COOKIE,
        refresh_token,
        max_age=settings.refresh_token_ttl_seconds,
        path=REFRESH_PATH,
        secure=True,
        httponly=True,
        samesite="strict",
    )
