from unittest.mock import MagicMock

from app.core.users import UserService


def test_register_saves_the_user():
    users = MagicMock()
    UserService(users).register("a@b.com")
    users.save.assert_called_once()
