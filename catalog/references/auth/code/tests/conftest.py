"""The settings, the clock, the identity provider and the stores of the tests."""

from __future__ import annotations

from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING

import jwt
import pytest
from cryptography.hazmat.primitives.asymmetric import rsa
from fastapi import FastAPI
from fastapi.testclient import TestClient
from jwt.algorithms import RSAAlgorithm
from sqlalchemy.pool import StaticPool
from sqlmodel import Session, create_engine

from app.api.dependencies import get_now, get_sessions
from app.api.v1.routers import auth, browser_session, orders
from app.core.sessions import Sessions
from app.database.services.migrations import upgrade_to_head
from app.impl.refresh_token_store import InMemoryRefreshTokenStore
from app.impl.settings import AuthSettings, get_settings

if TYPE_CHECKING:
    from collections.abc import Iterator

PROVIDER = "https://idp.example.com/"


class Clock:
    """The time of the requests, moved by the tests. PyJWT checks `exp` against the real time."""

    def __init__(self) -> None:
        self.now = datetime.now(UTC)

    def advance(self, delta: timedelta) -> None:
        self.now += delta


class Provider:
    """An identity provider: a key pair, the tokens it signs and the lookup of its public key."""

    def __init__(self) -> None:
        self.key = rsa.generate_private_key(public_exponent=65537, key_size=2048)

    def sign(self, sub: str, now: datetime, audience: str = "orders-web") -> str:
        claims = {"sub": sub, "iss": PROVIDER, "aud": audience, "iat": now}
        claims["exp"] = now + timedelta(minutes=5)
        return jwt.encode(claims, self.key, algorithm="RS256", headers={"kid": "k1"})

    def signing_key_of(self, token: str) -> jwt.PyJWK:
        public = RSAAlgorithm.to_jwk(self.key.public_key(), as_dict=True)
        return jwt.PyJWK({**public, "kid": "k1", "alg": "RS256"})


@pytest.fixture
def settings() -> AuthSettings:
    return AuthSettings(
        signing_key="a-test-key-of-thirty-two-bytes!!",
        issuer="https://api.example.com/",
        audience="orders",
        provider_issuer=PROVIDER,
        provider_audience="orders-web",
    )


@pytest.fixture
def clock() -> Clock:
    return Clock()


@pytest.fixture
def provider() -> Provider:
    return Provider()


@pytest.fixture
def database() -> Iterator[Session]:
    """Yield a session of a database in memory, built by the migrations and not by `create_all`."""
    engine = create_engine("sqlite://", poolclass=StaticPool)
    with engine.begin() as connection:
        upgrade_to_head(connection)
    with Session(engine) as session:
        yield session
    engine.dispose()


@pytest.fixture
def client(settings: AuthSettings, clock: Clock, provider: Provider) -> TestClient:
    app = FastAPI()
    app.include_router(auth.router, prefix="/v1")
    app.include_router(browser_session.router, prefix="/v1")
    app.include_router(orders.router, prefix="/v1")
    sessions = Sessions(InMemoryRefreshTokenStore(), settings.refresh_token_ttl_seconds)
    app.dependency_overrides[get_settings] = lambda: settings
    app.dependency_overrides[get_now] = lambda: clock.now
    app.dependency_overrides[get_sessions] = lambda: sessions
    app.dependency_overrides[auth.get_signing_key_of] = lambda: provider.signing_key_of
    return TestClient(app)
