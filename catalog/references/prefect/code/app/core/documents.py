"""The summary of a document: domain code, which knows nothing of Prefect."""

from __future__ import annotations

from pydantic import BaseModel

from app.core.logger import get_logger

log = get_logger(__name__)


class Document(BaseModel):
    """A document to summarize."""

    id: str
    text: str


class Summary(BaseModel):
    """What the summary of a document says about it."""

    document_id: str
    words: int
    first_sentence: str


def summarize(document: Document) -> Summary:
    """Return the summary of a document."""
    words = len(document.text.split())
    first_sentence = document.text.split(".", 1)[0].strip()
    log.info("document_summarized", document_id=document.id, words=words)
    return Summary(document_id=document.id, words=words, first_sentence=first_sentence)
