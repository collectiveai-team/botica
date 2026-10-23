"""The refresh tokens of a session, through each adapter of the port: in memory and SQL."""

from __future__ import annotations

from datetime import UTC, datetime, timedelta
from typing import TYPE_CHECKING

import pytest

from app.core.sessions import InvalidRefreshTokenError, Sessions, hash_token
from app.database.repositories.refresh_tokens import SqlRefreshTokenStore
from app.impl.refresh_token_store import InMemoryRefreshTokenStore

if TYPE_CHECKING:
    from app.core.sessions import RefreshTokenStore

NOW = datetime(2026, 10, 21, 12, 0, tzinfo=UTC)
FOURTEEN_DAYS = 14 * 24 * 3600


@pytest.fixture(params=["memory", "sql"])
def store(request: pytest.FixtureRequest) -> RefreshTokenStore:
    if request.param == "memory":
        return InMemoryRefreshTokenStore()
    return SqlRefreshTokenStore(request.getfixturevalue("database"))


@pytest.fixture
def sessions(store: RefreshTokenStore) -> Sessions:
    return Sessions(store, FOURTEEN_DAYS)


def test_a_refresh_token_is_replaced_by_a_new_one_for_the_same_user(sessions: Sessions):
    first = sessions.open("ana", NOW)

    user_id, second = sessions.rotate(first, NOW)

    assert user_id == "ana"
    assert second != first
    assert sessions.rotate(second, NOW)[0] == "ana"


def test_the_store_keeps_a_hash_and_not_the_token(store: RefreshTokenStore):
    token = Sessions(store, FOURTEEN_DAYS).open("ana", NOW)

    assert store.get(token) is None
    stored = store.get(hash_token(token))
    assert stored is not None
    assert (stored.user_id, stored.expires_at) == ("ana", NOW + timedelta(seconds=FOURTEEN_DAYS))


def test_a_refresh_token_used_twice_ends_the_session(sessions: Sessions):
    first = sessions.open("ana", NOW)
    _, second = sessions.rotate(first, NOW)

    with pytest.raises(InvalidRefreshTokenError):
        sessions.rotate(first, NOW)
    with pytest.raises(InvalidRefreshTokenError):
        sessions.rotate(second, NOW)


def test_a_token_is_marked_used_once(store: RefreshTokenStore):
    token = Sessions(store, FOURTEEN_DAYS).open("ana", NOW)

    assert [store.mark_used(hash_token(token)) for _ in range(2)] == [True, False]


def test_an_expired_refresh_token_is_invalid(sessions: Sessions):
    token = sessions.open("ana", NOW)

    with pytest.raises(InvalidRefreshTokenError):
        sessions.rotate(token, NOW + timedelta(seconds=FOURTEEN_DAYS))


def test_a_closed_session_refreshes_no_more(sessions: Sessions):
    token = sessions.open("ana", NOW)

    sessions.close(token)

    with pytest.raises(InvalidRefreshTokenError):
        sessions.rotate(token, NOW)
