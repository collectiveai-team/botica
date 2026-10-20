"""A fake of the repository of users: the whole port, kept in a dict, with no I/O."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from app.core.users import User


@dataclass
class InMemoryUserRepository:
    """The same interface as the SQL repository, kept in memory."""

    _by_email: dict[str, User] = field(default_factory=dict)

    def add(self, user: User) -> None:
        """Store a user."""
        self._by_email[user.email] = user

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        return self._by_email.get(email)

    def next_id(self) -> int:
        """Return an id that no stored user has."""
        return len(self._by_email) + 1
