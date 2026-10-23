from typing import TypeIs, cast as narrow

from app.core.users import User


def is_user(value: object) -> TypeIs[User]:
    return isinstance(value, User)


def load(payload: dict[str, object]) -> User:
    return narrow(User, payload["user"])
