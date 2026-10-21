"""The response schemas of the endpoints of documents."""

from __future__ import annotations

from uuid import UUID

from pydantic import BaseModel, ConfigDict


class SummarizeResponse(BaseModel):
    """The flow run that summarizes the documents."""

    model_config = ConfigDict(extra="forbid")

    flow_run_id: UUID
