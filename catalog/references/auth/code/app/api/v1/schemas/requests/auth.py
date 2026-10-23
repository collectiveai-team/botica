"""The request schemas of the endpoints of authentication."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


class SignInRequest(BaseModel):
    """The token that the identity provider gave the client at the end of its sign-in."""

    model_config = ConfigDict(extra="forbid")

    id_token: str
