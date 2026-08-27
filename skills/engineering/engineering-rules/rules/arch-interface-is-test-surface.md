---
title: Test through the same seam callers cross
section: arch
scope: general
applies-to: all
status: direction
tags: architecture, testing, seam, testability, matt-pocock
---

## Test through the same seam callers cross

Tests cross the same seam callers cross and assert observable behaviour — not internal
state. If a test must reach past the interface to inspect or mock something inside, the
module is the wrong shape: either the seam is in the wrong place, or the implementation
leaks internal structure that should be hidden.

**Prefer (test at the external seam — behaviour, not internals):**

```python
# Tests call store.search() exactly as callers do.
# ChromaStore's internal embedding logic, batching, and client calls are not touched.
def test_search_returns_top_k(vector_store: VectorStore):
    vector_store.upsert([Document(id="a", text="hello world")])
    results = vector_store.search("hello", k=1)
    assert len(results) == 1
    assert results[0].id == "a"
```

**Avoid (testing past the interface):**

```python
# Monkeypatching the internal _embed() call means the test knows about
# implementation details — it will break on any internal refactor.
def test_search_calls_embed(vector_store, monkeypatch):
    monkeypatch.setattr(vector_store, "_embed", lambda t: [0.1, 0.2])
    vector_store.search("hello", k=1)
```

Reference: Matt Pocock `codebase-design` — "The interface is the test surface. Callers
and tests cross the same seam." Tests that survive internal refactors describe behaviour,
not implementation. See `arch-deep-modules` for the full glossary and `test-through-interface`
for the testing-section counterpart.
