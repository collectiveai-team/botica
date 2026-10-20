"""The response schemas of the status endpoint."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict


class StatusResponse(BaseModel):
    """The state of the service."""

    model_config = ConfigDict(extra="forbid")

    debug: bool
