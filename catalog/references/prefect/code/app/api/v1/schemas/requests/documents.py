"""The request schemas of the endpoints of documents."""

from __future__ import annotations

from pydantic import BaseModel, ConfigDict, Field


class DocumentRequest(BaseModel):
    """A document to summarize."""

    model_config = ConfigDict(extra="forbid")

    id: str = Field(min_length=1)
    text: str


class SummarizeRequest(BaseModel):
    """The documents to summarize in one flow run."""

    model_config = ConfigDict(extra="forbid")

    documents: list[DocumentRequest] = Field(min_length=1)
