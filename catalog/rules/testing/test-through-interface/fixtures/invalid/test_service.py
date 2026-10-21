from app.core.users import CreateUser, UserService
from tests.fakes import InMemoryUserRepository


def test_register_writes_a_row():
    service = UserService(InMemoryUserRepository())
    service.register(CreateUser(email="a@b.com"))
    assert service._store._rows[0]["email"] == "a@b.com"
