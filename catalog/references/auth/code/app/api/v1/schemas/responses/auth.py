"""The response schemas of the endpoints of authentication."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


class AccessTokenResponse(BaseModel):
    """A new access token, and the seconds it lives."""

    model_config = ConfigDict(extra="forbid")

    access_token: str
    token_type: str = "bearer"  # noqa: S105 -- the scheme of RFC 6750, not a secret
    expires_in: int
