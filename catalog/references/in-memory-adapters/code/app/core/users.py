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

    def add(self, user: User) -> None:
        """Store a user."""
        ...

    def get_by_email(self, email: str) -> User | None:
        """Return the user with this email, or None."""
        ...

    def next_id(self) -> int:
        """Return an id that no stored user has."""
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
        user = User(id=self.users.next_id(), email=email)
        self.users.add(user)
        return user
