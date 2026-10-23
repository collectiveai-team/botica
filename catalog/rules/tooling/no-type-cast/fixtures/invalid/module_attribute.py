import typing

from app.core.users import User


def load(payload: dict[str, object]) -> User:
    return typing.cast("User", payload["user"])
