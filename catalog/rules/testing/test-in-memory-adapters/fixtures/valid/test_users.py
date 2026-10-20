from app.core.users import UserService
from tests.fakes import InMemoryUserRepository


def test_a_registered_user_is_found_by_email():
    users = InMemoryUserRepository()
    UserService(users).register("a@b.com")
    assert users.get_by_email("a@b.com") is not None
