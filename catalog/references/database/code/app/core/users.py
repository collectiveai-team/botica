"""The users of the domain and the port of the store that keeps them."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class User:
    """A registered user."""

    id: int
    email: str


class UserRepository(Protocol):
    """The port. `SqlUserRepository` of `app.database` and the fakes of the tests implement it."""

    def add(self, email: str) -> User:
        """Store a user with this email and return it with the id the store gave it."""
        ...

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        ...
