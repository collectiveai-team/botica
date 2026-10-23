from typing import TypeIs

from app.core.users import User


def is_user(value: object) -> TypeIs[User]:
    return isinstance(value, User)


def load(payload: dict[str, object]) -> User:
    user = payload["user"]
    if not is_user(user):
        raise TypeError("payload['user'] is not a User")
    return user
