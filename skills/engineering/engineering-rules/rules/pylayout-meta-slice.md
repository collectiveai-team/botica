---
title: meta/ holds ABCs and Pydantic domain models — keep business logic out of it
section: pylayout
scope: general
applies-to: all
status: current
tags: layout, meta, abc, interfaces, pydantic, domain-models
---

## meta/ holds ABCs and Pydantic domain models — keep business logic out of it

Every package with pluggable backends uses a `meta/` slice for two things only:
abstract base classes (the seam callers depend on) and Pydantic domain models
(the shared data shapes). Concrete implementations and orchestration logic live
in sibling slices — never in `meta/`.

**Prefer:**

```python
# meta/interfaces/vector_store.py
from abc import ABC, abstractmethod
from mypackage.meta.retrieve_document import RetrievedDocument

class VectorStore(ABC):
    @abstractmethod
    def search(self, query: str, k: int) -> list[RetrievedDocument]: ...
```

**Avoid:**

```python
# Don't put Chroma/Weaviate calls or orchestration inside meta/
# meta/ is the seam, not the implementation
class VectorStore(ABC):
    def search(self, query: str, k: int) -> list[RetrievedDocument]:
        client = chromadb.Client()   # concrete logic bleeds into the interface
        ...
```

Cross-ref: `pylayout-adapters-slices` for where the concrete implementations go.
