"""A fake of the repository of users: the whole port, kept in a dict, with no I/O."""

from __future__ import annotations

from dataclasses import dataclass, field
from itertools import count

from app.core.users import User


@dataclass
class InMemoryUserRepository:
    """The same interface as the SQL repository, kept in memory."""

    _by_email: dict[str, User] = field(default_factory=dict)
    _ids: count[int] = field(default_factory=lambda: count(1))

    def add(self, email: str) -> User:
        """Store a user with this email and return it with the id the fake gave it."""
        user = User(id=next(self._ids), email=email)
        self._by_email[email] = user
        return user

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        return self._by_email.get(email)
