"""The SQL adapter of the port `UserRepository`."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from sqlmodel import select

from app.core.users import User
from app.database.models.users import UserRow

if TYPE_CHECKING:
    from sqlmodel import Session


@dataclass
class SqlUserRepository:
    """Keeps the users in the table `users`, and returns them as `User` of `app.core`."""

    session: Session

    def add(self, email: str) -> User:
        """Store a user with this email and return it with the id the database gave it."""
        row = UserRow(email=email)
        self.session.add(row)
        self.session.commit()
        self.session.refresh(row)
        return _to_user(row)

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        row = self.session.exec(select(UserRow).where(UserRow.email == email)).first()
        return None if row is None else _to_user(row)


def _to_user(row: UserRow) -> User:
    if row.id is None:
        raise ValueError("the database gave the row no id")
    return User(id=row.id, email=row.email)
