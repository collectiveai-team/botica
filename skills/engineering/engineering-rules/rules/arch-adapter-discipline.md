---
title: Don't add a seam until two adapters justify it
section: arch
scope: general
applies-to: all
status: direction
tags: architecture, adapters, seam, matt-pocock
---

## Don't add a seam until two adapters justify it

One adapter means a hypothetical seam. Two adapters means a real one. Don't introduce a
port, registry, factory, or ABC until something actually varies across the seam in
production — typically a production adapter plus at least one of: a test/local adapter,
a second real backend, or a variant environment. A single-adapter seam is indirection
without leverage.

**Prefer (two real adapters — seam is justified):**

```python
# vector_stores/ — two production backends, seam is real
class ChromaStore(VectorStore):   # production: local / self-hosted
    ...

class WeaviateStore(VectorStore): # production: managed Weaviate cluster
    ...
```

```python
# adapters/ — multiple real backends behind one contract
# azure.py, gcp.py, inmemory.py, sqs.py
# Each fills a different deployment slot; the seam pays for itself.
```

**Avoid (one adapter — seam is hypothetical):**

```python
# Introducing an ABC + a single concrete class "for future flexibility"
# adds interface complexity with no current leverage.
class NotificationSender(ABC):
    def send(self, msg: str) -> None: ...

class SlackSender(NotificationSender):  # only ever one
    def send(self, msg: str) -> None: ...
```

Reference: Matt Pocock `codebase-design` skill — "One adapter means a hypothetical seam.
Two adapters means a real one." See `arch-deep-modules` for the full glossary and
`arch-deletion-test` for the companion decision heuristic.
