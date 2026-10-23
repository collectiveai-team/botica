"""The registration of users: a service and the port of the store it depends on."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class User:
    """A registered user."""

    id: int
    email: str


class UserRepository(Protocol):
    """The port. The SQL adapter and the fake of the tests both implement it."""

    def add(self, email: str) -> User:
        """Store a user with this email and return it with the id the store gave it."""
        ...

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        ...


class EmailTakenError(Exception):
    """A user with this email is already registered."""


@dataclass
class UserService:
    """Registers users in the repository it is given."""

    users: UserRepository

    def register(self, email: str) -> User:
        """Register a user with this email, in lower case; fail when the email is taken."""
        email = email.lower()
        if self.users.get_by_email(email) is not None:
            raise EmailTakenError(email)
        return self.users.add(email)
