"""The structures that cross the boundaries of the domain."""

from __future__ import annotations

from dataclasses import dataclass

from pydantic import BaseModel


@dataclass(frozen=True)
class Point:
    """A point of the plane: a dataclass, since nothing in it needs validation."""

    x: int
    y: int


def origin() -> Point:
    """Return the origin of the plane."""
    return Point(x=0, y=0)


class NewUser(BaseModel):
    """A user before it is stored: a model of the domain whose fields are validated."""

    email: str
    display_name: str


def normalize(user: NewUser) -> NewUser:
    """Return the user with the email in lower case and the name without surrounding spaces."""
    return NewUser(email=user.email.lower(), display_name=user.display_name.strip())
