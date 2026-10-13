"""CES-4 · api-schemas-extra-forbid — compliant API request/response schemas (Pydantic v2).

Drop-in / comparison template for schemas living under
``<pkg>/api/<version>/schemas/{requests,responses}/``. Every request and response model at the
API boundary sets ``model_config = ConfigDict(extra="forbid")`` so an unknown field (a typo, a
stale client, an injected key) is rejected with a 422 instead of being silently ignored.

Internal/domain models are NOT subject to this rule — it is scoped by placement to the api/
schema packages. See ``uvx botica explain api-schemas-extra-forbid`` for the full rule.
"""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict, EmailStr, Field


class CreateUserRequest(BaseModel):
    """Inbound payload — reject anything not declared here."""

    model_config = ConfigDict(extra="forbid")

    email: EmailStr
    display_name: str = Field(min_length=1, max_length=80)


class UserResponse(BaseModel):
    """Outbound payload — forbid extras so the contract stays exactly what we document."""

    model_config = ConfigDict(extra="forbid")

    id: int
    email: EmailStr
    display_name: str


# Anti-pattern (flagged by CES-4) — a boundary schema with no extra="forbid":
#
#     class CreateUserRequest(BaseModel):   # api-schemas-extra-forbid
#         email: str
