"""The access tokens of the service: the lifetime of the settings, and a 401 for a bad one."""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import jwt
import pytest
from pydantic import ValidationError

from app.impl.access_tokens import issue_access_token
from app.impl.settings import AuthSettings

if TYPE_CHECKING:
    from fastapi.testclient import TestClient

    from tests.conftest import Clock


def _bearer(token: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {token}"}


def test_an_access_token_lives_900_seconds_unless_the_environment_says_otherwise(
    settings: AuthSettings, monkeypatch: pytest.MonkeyPatch
):
    assert settings.access_token_ttl_seconds == 900

    monkeypatch.setenv("APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", "3600")
    assert AuthSettings(**_required(settings)).access_token_ttl_seconds == 3600


def test_the_settings_reject_a_lifetime_that_is_not_above_0(settings: AuthSettings):
    with pytest.raises(ValidationError, match="greater than 0"):
        AuthSettings.model_validate({**_required(settings), "access_token_ttl_seconds": 0})


def _required(settings: AuthSettings) -> dict[str, str]:
    return {
        "signing_key": settings.signing_key.get_secret_value(),
        "issuer": settings.issuer,
        "audience": settings.audience,
    }


def test_an_access_token_expires_after_the_lifetime_of_the_settings(
    settings: AuthSettings, clock: Clock
):
    token = issue_access_token("ana", settings, clock.now)

    claims = jwt.decode(
        token,
        settings.signing_key.get_secret_value(),
        algorithms=["HS256"],
        audience=settings.audience,
    )

    assert claims["exp"] - claims["iat"] == settings.access_token_ttl_seconds


def test_a_valid_access_token_reaches_the_route(
    client: TestClient, settings: AuthSettings, clock: Clock
):
    response = client.get(
        "/v1/orders", headers=_bearer(issue_access_token("ana", settings, clock.now))
    )

    assert response.status_code == 200
    assert response.json() == {"user_id": "ana", "orders": []}


def test_a_route_without_a_token_is_a_401(client: TestClient):
    response = client.get("/v1/orders")

    assert response.status_code == 401
    assert response.headers["www-authenticate"] == "Bearer"


def test_an_expired_access_token_is_a_401(client: TestClient, settings: AuthSettings, clock: Clock):
    token = issue_access_token("ana", settings, clock.now - timedelta(hours=1))

    assert client.get("/v1/orders", headers=_bearer(token)).status_code == 401


def test_an_access_token_signed_with_another_key_is_a_401(
    client: TestClient, settings: AuthSettings, clock: Clock
):
    other = AuthSettings.model_validate({**settings.model_dump(), "signing_key": "x" * 32})

    response = client.get(
        "/v1/orders", headers=_bearer(issue_access_token("ana", other, clock.now))
    )

    assert response.status_code == 401


def test_a_public_route_needs_no_token(client: TestClient):
    assert client.get("/v1/health").status_code == 200
