---
title: Apply the deletion test before adding a module
section: arch
scope: general
applies-to: all
status: direction
tags: architecture, deep-modules, refactor, matt-pocock
---

## Apply the deletion test before adding a module

Before introducing a module, wrapper, or layer, imagine deleting it. If the complexity
**vanishes**, it was a pass-through and should not exist. If the complexity **reappears
spread across N callers**, the module is earning its place — it concentrates logic that
would otherwise leak everywhere. This is the primary signal for telling a deep module from
a shallow one.

**Prefer (deep — deleting it would spread complexity across callers):**

```python
# vector_stores/chroma_store.py — callers say `store.search(query, k=5)`;
# batching, embedding cache, distance conversion, and the Chroma client all hide here.
class ChromaStore(VectorStore):
    def search(self, query: str, k: int) -> list[RetrievedDocument]: ...
```

**Avoid (shallow — deleting it changes nothing but one import):**

```python
# helpers/search_helper.py
def do_search(store, query, k):
    return store.search(query, k)  # pass-through; interface as complex as the body
```

Reference: Matt Pocock `codebase-design` skill (github.com/mattpocock/skills). The deletion
test and depth-as-leverage framing are his; he explicitly rejects Ousterhout's
implementation-lines-to-interface-lines ratio. See `arch-deep-modules` for the full glossary
and `spaghetti-no-utils-dumping` for the shallow pass-through anti-pattern.
