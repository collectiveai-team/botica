"""The request schemas of the endpoints of users."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict, Field


class CreateUserRequest(BaseModel):
    """The payload that registers a user."""

    model_config = ConfigDict(extra="forbid")

    email: str = Field(min_length=3, max_length=254)
