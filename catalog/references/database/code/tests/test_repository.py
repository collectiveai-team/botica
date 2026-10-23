"""The SQL adapter of the port, through the methods of the port."""

from __future__ import annotations

from typing import TYPE_CHECKING

from app.core.users import User
from app.database.repositories.users import SqlUserRepository

if TYPE_CHECKING:
    from sqlmodel import Session


def test_add_returns_a_user_of_the_domain_with_the_id_of_the_database(session: Session):
    user = SqlUserRepository(session).add("ana@example.com")

    assert user == User(id=1, email="ana@example.com")


def test_a_stored_user_is_found_by_its_email(session: Session):
    users = SqlUserRepository(session)
    stored = users.add("ana@example.com")

    assert users.get_by_email("ana@example.com") == stored


def test_an_unknown_email_is_none(session: Session):
    assert SqlUserRepository(session).get_by_email("nobody@example.com") is None
