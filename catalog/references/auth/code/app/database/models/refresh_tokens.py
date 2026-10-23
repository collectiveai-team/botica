"""The table of refresh tokens."""

from __future__ import annotations

from datetime import datetime

import sqlalchemy as sa
from sqlmodel import Field, SQLModel


class RefreshTokenRow(SQLModel, table=True):
    """A row of `refresh_tokens`: the hash of a token and its session, never the token."""

    # SQLModel types `__tablename__` as a declared attribute; SQLAlchemy reads a plain string too.
    __tablename__ = "refresh_tokens"  # pyrefly: ignore[bad-override]

    token_hash: str = Field(primary_key=True, max_length=64)
    user_id: str = Field(max_length=255)
    family: str = Field(max_length=64, index=True)
    expires_at: datetime = Field(sa_type=sa.DateTime(timezone=True))
    used: bool = False
