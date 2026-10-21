"""The response schemas of the endpoints of users."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


class UserResponse(BaseModel):
    """A user as the API returns it."""

    model_config = ConfigDict(extra="forbid")

    id: int
    email: str
    display_name: str
