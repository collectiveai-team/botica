"""The response schemas of the endpoints of users."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic import BaseModel, ConfigDict

if TYPE_CHECKING:
    from app.core.users import User


class UserResponse(BaseModel):
    """A user as the API returns it."""

    model_config = ConfigDict(extra="forbid")

    id: int
    email: str

    @classmethod
    def of(cls, user: User) -> UserResponse:
        """Build the response from a user of the domain, one field at a time."""
        return cls(id=user.id, email=user.email)
