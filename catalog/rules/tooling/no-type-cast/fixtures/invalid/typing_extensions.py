from typing_extensions import (
    TypeIs,
    cast,
)

from app.core.users import User


def is_user(value: object) -> TypeIs[User]:
    return isinstance(value, User)


def load(payload: dict[str, object]) -> User:
    return cast(User, payload["user"])
