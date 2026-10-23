from app.core.users import User


def load(payload: dict[str, object]) -> User:
    from typing import cast

    return cast(User, payload["user"])
