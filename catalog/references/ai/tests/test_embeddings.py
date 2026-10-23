"""The embedder of the topic, driven by the test embedding model of pydantic-ai."""

from __future__ import annotations

import asyncio

from pydantic_ai.embeddings import TestEmbeddingModel

from app.core.embeddings import embed_passages, embed_question, passage_embedder

PASSAGES = ["The buyer pays on delivery.", "The seller keeps the title until payment."]


def test_each_passage_gets_one_vector_of_the_dimensions_of_the_model():
    embedder = passage_embedder(TestEmbeddingModel(dimensions=4))

    vectors = asyncio.run(embed_passages(embedder, PASSAGES))

    assert [len(vector) for vector in vectors] == [4, 4]


def test_a_question_gets_one_vector_of_the_same_dimensions():
    embedder = passage_embedder(TestEmbeddingModel(dimensions=4))

    assert len(asyncio.run(embed_question(embedder, "When does the buyer pay?"))) == 4


def test_a_test_replaces_the_model_that_the_settings_name():
    embedder = passage_embedder("openai:text-embedding-3-small")

    with embedder.override(model=TestEmbeddingModel(dimensions=2)):
        vectors = asyncio.run(embed_passages(embedder, PASSAGES))

    assert vectors == [[1.0, 1.0], [1.0, 1.0]]
