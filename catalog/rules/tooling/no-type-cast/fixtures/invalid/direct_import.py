from typing import cast

from app.core.users import User


def load(payload: dict[str, object]) -> User:
    return cast(User, payload["user"])
