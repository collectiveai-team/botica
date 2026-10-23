"""The refresh tokens of a session: stored as a hash, replaced at each use, revoked when reused."""

from __future__ import annotations

import hashlib
import secrets
from dataclasses import dataclass
from datetime import timedelta
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from datetime import datetime


@dataclass(frozen=True)
class StoredRefreshToken:
    """What the server keeps of a refresh token: never the token itself."""

    user_id: str
    family: str
    expires_at: datetime
    used: bool = False


class RefreshTokenStore(Protocol):
    """The port of the store of refresh tokens, keyed by the hash of each token.

    `InMemoryRefreshTokenStore` of `app.impl` and `SqlRefreshTokenStore` of `app.database`
    implement it.
    """

    def add(self, token_hash: str, stored: StoredRefreshToken) -> None:
        """Store a new refresh token under its hash."""
        ...

    def get(self, token_hash: str) -> StoredRefreshToken | None:
        """Return what is stored for the hash, or None."""
        ...

    def mark_used(self, token_hash: str) -> bool:
        """Mark the token as used, and return False when it was used already.

        One step, so that two requests with the same token cannot both see it unused.
        """
        ...

    def delete_family(self, family: str) -> None:
        """Delete every refresh token of a session."""
        ...


class InvalidRefreshTokenError(Exception):
    """The refresh token is unknown, expired or used already; the caller signs in again."""


def hash_token(token: str) -> str:
    """Return the SHA-256 of a refresh token, the key of the store."""
    return hashlib.sha256(token.encode()).hexdigest()


@dataclass
class Sessions:
    """Opens sessions, rotates their refresh tokens and ends them."""

    store: RefreshTokenStore
    refresh_token_ttl_seconds: int

    def open(self, user_id: str, now: datetime) -> str:
        """Start a session for the user and return its first refresh token."""
        return self._issue(user_id, secrets.token_urlsafe(16), now)

    def rotate(self, token: str, now: datetime) -> tuple[str, str]:
        """Return the user of the token and the refresh token that replaces it.

        A token that was used already ends its session: the caller or a thief holds a copy.
        """
        token_hash = hash_token(token)
        stored = self.store.get(token_hash)
        if stored is None or stored.expires_at <= now:
            raise InvalidRefreshTokenError
        if not self.store.mark_used(token_hash):
            self.store.delete_family(stored.family)
            raise InvalidRefreshTokenError
        return stored.user_id, self._issue(stored.user_id, stored.family, now)

    def close(self, token: str) -> None:
        """End the session of the refresh token, if it has one."""
        stored = self.store.get(hash_token(token))
        if stored is not None:
            self.store.delete_family(stored.family)

    def _issue(self, user_id: str, family: str, now: datetime) -> str:
        token = secrets.token_urlsafe(32)
        expires_at = now + timedelta(seconds=self.refresh_token_ttl_seconds)
        self.store.add(hash_token(token), StoredRefreshToken(user_id, family, expires_at))
        return token
