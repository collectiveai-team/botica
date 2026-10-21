"""The request schemas of the endpoints of users."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict, Field


class CreateUserRequest(BaseModel):
    """The payload that creates a user. A key that the model does not declare is a 422."""

    model_config = ConfigDict(extra="forbid")

    email: str = Field(min_length=3, max_length=254)
    display_name: str = Field(min_length=1, max_length=80)
