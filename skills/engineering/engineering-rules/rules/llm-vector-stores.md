---
title: Put vector stores behind an ABC; cache embeddings through the same store
section: llm
scope: specific
applies-to: llm
status: current
tags: llm, vector-store, embeddings, chroma, weaviate, abc, cache
---

## Put vector stores behind an ABC; cache embeddings through the same store

All vector-store access goes through a `VectorStore` ABC. Concrete stores
(`ChromaStore`, `WeaviateStore`) are injected, not instantiated inline. The
`TextEncoder.encode()` method reuses the same `VectorStore` as an **embedding
cache**: it loads cached vectors first, calls `_encode()` only for unseen
documents, then saves the new vectors back. `Document` is content-addressable
(UUIDv5 derived from text), so cache hits are automatic.

**Prefer:**

```python
# interfaces/text_encoder.py — embedding cache pattern
def encode(self, documents: list[Document]) -> np.ndarray:
    if self.embedding_cache is None:
        return self._encode(documents)

    loaded = self.embedding_cache.load(documents)          # hits by Document.id
    uncached = [d for d, v in zip(documents, loaded) if v is None]
    if uncached:
        new_vectors = self._encode(uncached)
        self.embedding_cache.save(uncached, new_vectors)   # persist for next run
    ...
```

```python
# wiring encoder + cache together
from your_package.vector_stores.chroma_store import ChromaStore
from your_package.encoders.ollama_text_encoder import OllamaTextEncoder

vector_store = ChromaStore(
    collection_name="ollama_embeddings",
    persist_directory="/var/cache/chroma_db",
)
encoder = OllamaTextEncoder(model_name="mxbai-embed-large", embedding_cache=vector_store)
```

**Avoid:**

```python
# Instantiating a store directly in business logic, bypassing the ABC
import chromadb
client = chromadb.PersistentClient(path="./db")   # no abstraction, no swappability
```

**Design notes:**

- Define ABCs for `VectorStore`, `TextEncoder`, and `Retrieval` as separate interfaces.
- Concrete stores: `ChromaStore` (local/dev) and `WeaviateStore` (production scale).
- `Document` uses `generate_uuid5(text)` for content-addressable IDs — this makes
  cache hits automatic without explicit key management.
- `TextEncoder` accepts `embedding_cache: VectorStore | None`; pass `None` to skip
  caching in tests.
- For hybrid retrieval, combine the vector store with BM25 (`rank-bm25`) and
  semantic routing (`semantic-router`).
- Use `tiktoken` for token counting when working with OpenAI embedding models.

Reference: `llm-diskcache` for the separate LLM-response cache.
