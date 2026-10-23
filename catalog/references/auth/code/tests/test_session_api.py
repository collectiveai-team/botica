"""Sign in with a token of the provider, refresh and sign out: token in the body, and in cookies."""

from __future__ import annotations

from http.cookies import SimpleCookie
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from fastapi.testclient import TestClient
    from httpx2 import Response

    from tests.conftest import Clock, Provider


def cookies_of(response: Response) -> SimpleCookie:
    cookie = SimpleCookie()
    for header in response.headers.get_list("set-cookie"):
        cookie.load(header)
    return cookie


def _sent(token: str, name: str = "refresh_token") -> dict[str, str]:
    # The cookies are `Secure`, and the test client speaks plain HTTP, so the test sends them by
    # hand.
    return {"Cookie": f"{name}={token}"}


def sign_in(client: TestClient, provider: Provider, clock: Clock) -> Response:
    return client.post("/v1/auth/session", json={"id_token": provider.sign("ana", clock.now)})


def test_signing_in_returns_an_access_token_and_sets_the_cookies_of(
    client: TestClient, provider: Provider, clock: Clock
):
    response = sign_in(client, provider, clock)

    assert response.status_code == 200
    assert response.json()["expires_in"] == 900
    morsel = cookies_of(response)["refresh_token"]
    assert (morsel["httponly"], morsel["secure"], morsel["samesite"]) == (True, True, "strict")
    assert morsel["path"] == "/v1/auth"


def test_the_access_token_of_the_session_reaches_the_route(
    client: TestClient, provider: Provider, clock: Clock
):
    access_token = sign_in(client, provider, clock).json()["access_token"]

    response = client.get("/v1/orders", headers={"Authorization": f"Bearer {access_token}"})

    assert response.json()["user_id"] == "ana"


def test_a_token_of_the_provider_for_another_audience_is_a_401(
    client: TestClient, provider: Provider, clock: Clock
):
    token = provider.sign("ana", clock.now, audience="another-app")

    assert client.post("/v1/auth/session", json={"id_token": token}).status_code == 401


def test_a_refresh_replaces_the_cookie_and_a_replayed_cookie_ends_the_session(
    client: TestClient, provider: Provider, clock: Clock
):
    first = cookies_of(sign_in(client, provider, clock))["refresh_token"].value

    refreshed = client.post("/v1/auth/refresh", headers=_sent(first))
    second = cookies_of(refreshed)["refresh_token"].value

    assert refreshed.status_code == 200
    assert second != first
    assert client.post("/v1/auth/refresh", headers=_sent(first)).status_code == 401
    assert client.post("/v1/auth/refresh", headers=_sent(second)).status_code == 401


def test_signing_out_ends_the_session(client: TestClient, provider: Provider, clock: Clock):
    token = cookies_of(sign_in(client, provider, clock))["refresh_token"].value

    assert client.post("/v1/auth/logout", headers=_sent(token)).status_code == 204
    assert client.post("/v1/auth/refresh", headers=_sent(token)).status_code == 401


def sign_in_browser(client: TestClient, provider: Provider, clock: Clock) -> Response:
    return client.post("/v1/browser/session", json={"id_token": provider.sign("ana", clock.now)})


def test_a_browser_session_puts_both_tokens_in_httponly_cookies(
    client: TestClient, provider: Provider, clock: Clock
):
    response = sign_in_browser(client, provider, clock)

    assert response.status_code == 204
    assert response.content == b""
    cookies = cookies_of(response)
    access, refresh = cookies["__Host-access"], cookies["__Secure-refresh"]
    assert (access["path"], access["max-age"]) == ("/", "900")
    assert refresh["path"] == "/v1/browser"
    for morsel in (access, refresh):
        assert (morsel["httponly"], morsel["secure"], morsel["samesite"]) == (True, True, "strict")


def test_the_access_cookie_reaches_the_route(client: TestClient, provider: Provider, clock: Clock):
    access = cookies_of(sign_in_browser(client, provider, clock))["__Host-access"].value

    response = client.get("/v1/orders", headers=_sent(access, "__Host-access"))

    assert response.json()["user_id"] == "ana"


def test_a_browser_refresh_replaces_both_cookies_and_a_replayed_one_ends_the_session(
    client: TestClient, provider: Provider, clock: Clock
):
    first = cookies_of(sign_in_browser(client, provider, clock))["__Secure-refresh"].value

    refreshed = client.post("/v1/browser/refresh", headers=_sent(first, "__Secure-refresh"))

    assert refreshed.status_code == 204
    assert set(cookies_of(refreshed)) == {"__Host-access", "__Secure-refresh"}
    replayed = client.post("/v1/browser/refresh", headers=_sent(first, "__Secure-refresh"))
    assert replayed.status_code == 401
