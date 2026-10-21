from app.core.users import CreateUser, UserService
from tests.fakes import InMemoryUserRepository


def test_a_registered_user_is_found():
    service = UserService(InMemoryUserRepository())
    result = service.register(CreateUser(email="a@b.com"))
    assert service.find(result.id).email == "a@b.com"
