"""The SQL adapter of the port `RefreshTokenStore`."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC
from typing import TYPE_CHECKING

from sqlmodel import col, delete, update

from app.core.sessions import StoredRefreshToken
from app.database.models.refresh_tokens import RefreshTokenRow

if TYPE_CHECKING:
    from sqlmodel import Session


@dataclass
class SqlRefreshTokenStore:
    """Keeps the refresh tokens in the table `refresh_tokens`, shared by every process."""

    session: Session

    def add(self, token_hash: str, stored: StoredRefreshToken) -> None:
        """Store a new refresh token under its hash."""
        self.session.add(
            RefreshTokenRow(
                token_hash=token_hash,
                user_id=stored.user_id,
                family=stored.family,
                expires_at=stored.expires_at,
                used=stored.used,
            )
        )
        self.session.commit()

    def get(self, token_hash: str) -> StoredRefreshToken | None:
        """Return what is stored for the hash, or None."""
        row = self.session.get(RefreshTokenRow, token_hash)
        return None if row is None else _to_stored(row)

    def mark_used(self, token_hash: str) -> bool:
        """Mark the token as used, and return False when it was used already.

        One `UPDATE ... WHERE used IS false`: of two requests with the same token, the database
        lets one change the row.
        """
        result = self.session.exec(
            update(RefreshTokenRow)
            .where(col(RefreshTokenRow.token_hash) == token_hash)
            .where(col(RefreshTokenRow.used).is_(False))
            .values(used=True)
        )
        self.session.commit()
        return result.rowcount == 1

    def delete_family(self, family: str) -> None:
        """Delete every refresh token of a session."""
        self.session.exec(delete(RefreshTokenRow).where(col(RefreshTokenRow.family) == family))
        self.session.commit()


def _to_stored(row: RefreshTokenRow) -> StoredRefreshToken:
    expires_at = row.expires_at
    if expires_at.tzinfo is None:
        # SQLite keeps no time zone; the column holds UTC.
        expires_at = expires_at.replace(tzinfo=UTC)
    return StoredRefreshToken(row.user_id, row.family, expires_at, row.used)
