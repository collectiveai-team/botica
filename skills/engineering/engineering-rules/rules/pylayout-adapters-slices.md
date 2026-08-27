---
title: Concrete implementations live in sibling slices — one backend per file
section: pylayout
scope: general
applies-to: all
status: current
tags: layout, adapters, slices, abc, backends
---

## Concrete implementations live in sibling slices — one backend per file

Implementations of the ABCs defined in `meta/` live in sibling slices (directories or
flat files), each in its own file, each subclassing the relevant ABC. Adding a new
backend means adding a new file — not editing `meta/` or stuffing multiple backends
into one file.

**Prefer:**

```python
# vector_stores/chroma_store.py
from mypackage.meta.interfaces.vector_store import VectorStore

class ChromaStore(VectorStore):
    def search(self, query: str, k: int) -> list[RetrievedDocument]: ...

# vector_stores/weaviate_store.py
class WeaviateStore(VectorStore):
    def search(self, query: str, k: int) -> list[RetrievedDocument]: ...
```

**Avoid:**

```python
# Don't pile multiple backends into one file
# vector_stores/stores.py
class ChromaStore(VectorStore): ...
class WeaviateStore(VectorStore): ...   # hard to delete, hard to test independently
```

For example, a well-structured adapters directory might contain `inmemory.py`, `sqs.py`,
`gcp.py`, `azure.py` — each adapter in its own file, each subclassing the same ABC.
Similarly, LLM client wrappers belong in separate files per provider.

Cross-ref: `pylayout-meta-slice` for the ABC home; `arch-deletion-test` for why one
backend per file earns its existence.
