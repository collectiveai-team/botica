"""CES-64 · test-in-memory-adapters — a working in-memory port implementation.

A fake is a *real* implementation of the same Protocol the production adapter implements — not a
mock. Tests drive logic through it (CES-65: through the interface), assert on outcomes, and stay
millisecond-fast and deterministic.

Drop this beside your tests (e.g. ``tests/fakes/in_memory_repository.py``) and inject it where the
real SQL/HTTP adapter would go. See CES-64 for the rule.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol


@dataclass(frozen=True)
class User:
    id: int
    email: str


class UserRepository(Protocol):
    """The port. The SQL adapter and this fake both implement it."""

    def add(self, user: User) -> None: ...

    def get_by_email(self, email: str) -> User | None: ...


@dataclass
class InMemoryUserRepository:
    """In-memory adapter — same interface as the real repository, no I/O."""

    _by_email: dict[str, User] = field(default_factory=dict)

    def add(self, user: User) -> None:
        self._by_email[user.email] = user

    def get_by_email(self, email: str) -> User | None:
        return self._by_email.get(email)


# Usage in a test (assert on behaviour through the public interface, CES-65):
#
#     def test_register_persists_user() -> None:
#         repo = InMemoryUserRepository()
#         service = UserService(repo)            # production logic, fake port
#         service.register("a@b.com")
#         assert repo.get_by_email("a@b.com") is not None
