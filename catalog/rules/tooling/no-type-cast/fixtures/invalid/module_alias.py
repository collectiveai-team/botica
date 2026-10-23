import typing as t

from app.core.users import User


def load(payload: dict[str, object]) -> User:
    return t.cast(User, payload["user"])
