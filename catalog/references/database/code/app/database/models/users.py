"""The table of users."""

from __future__ import annotations

from sqlmodel import Field, SQLModel


class UserRow(SQLModel, table=True):
    """A row of `users`. It stays in `app.database`; the repository turns it into a `User`."""

    # SQLModel types `__tablename__` as a declared attribute; SQLAlchemy reads a plain string too.
    __tablename__ = "users"  # pyrefly: ignore[bad-override]

    id: int | None = Field(default=None, primary_key=True)
    email: str = Field(max_length=254, unique=True, index=True)
