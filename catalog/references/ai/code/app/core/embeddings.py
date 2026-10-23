"""The vectors of passages of text, from an embedding model, to search them by meaning."""

from __future__ import annotations

from typing import TYPE_CHECKING

from pydantic_ai import Embedder

if TYPE_CHECKING:
    from collections.abc import Sequence

    from pydantic_ai.embeddings import EmbeddingModel


def passage_embedder(model: EmbeddingModel | str) -> Embedder:
    """Return the embedder of the passages, with the embedding model the settings name."""
    return Embedder(model)


async def embed_passages(embedder: Embedder, passages: Sequence[str]) -> list[Sequence[float]]:
    """Return one vector per passage, in the order of the passages."""
    result = await embedder.embed_documents(passages)
    return list(result.embeddings)


async def embed_question(embedder: Embedder, question: str) -> Sequence[float]:
    """Return the vector of a question, to compare it with the vectors of the passages."""
    result = await embedder.embed_query(question)
    return result.embeddings[0]
