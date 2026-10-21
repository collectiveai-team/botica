"""The service of users, driven through the fake of its repository."""

from __future__ import annotations

import pytest
from tests.fakes.users import InMemoryUserRepository

from app.core.users import EmailTakenError, UserService


def test_register_stores_the_user_with_its_email_in_lower_case():
    users = InMemoryUserRepository()

    user = UserService(users).register("Ana@Example.com")

    assert users.get_by_email("ana@example.com") == user


def test_an_email_is_registered_once():
    service = UserService(InMemoryUserRepository())
    service.register("ana@example.com")

    with pytest.raises(EmailTakenError):
        service.register("ANA@example.com")
