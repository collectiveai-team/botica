"""A store of refresh tokens in the memory of one process."""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from app.core.sessions import StoredRefreshToken


@dataclass
class InMemoryRefreshTokenStore:
    """The port `RefreshTokenStore` in a dict: for one process, and for the tests.

    Several processes share the tokens through `SqlRefreshTokenStore` of `app.database`.
    """

    _by_hash: dict[str, StoredRefreshToken] = field(default_factory=dict)

    def add(self, token_hash: str, stored: StoredRefreshToken) -> None:
        """Store a new refresh token under its hash."""
        self._by_hash[token_hash] = stored

    def get(self, token_hash: str) -> StoredRefreshToken | None:
        """Return what is stored for the hash, or None."""
        return self._by_hash.get(token_hash)

    def mark_used(self, token_hash: str) -> bool:
        """Mark the token as used, and return False when it was used already."""
        stored = self._by_hash.get(token_hash)
        if stored is None or stored.used:
            return False
        self._by_hash[token_hash] = replace(stored, used=True)
        return True

    def delete_family(self, family: str) -> None:
        """Delete every refresh token of a session."""
        for token_hash in [h for h, s in self._by_hash.items() if s.family == family]:
            del self._by_hash[token_hash]
