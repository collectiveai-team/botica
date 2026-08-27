---
title: Design deep modules — a lot of behaviour behind a small interface
section: arch
scope: general
applies-to: all
status: direction
tags: architecture, deep-modules, glossary, matt-pocock
---

## Design deep modules — a lot of behaviour behind a small interface

The shared architecture vocabulary. Use these terms exactly; consistent language is the
point. The goal is **deep modules**: maximum leverage for callers, maximum locality for
maintainers, and a test surface that matches the caller surface.

### Glossary

**Module** — anything with an interface and an implementation. Deliberately scale-agnostic:
a function, class, package, or tier-spanning slice. Avoid: unit, component, service.

**Interface** — everything a caller must know to use the module correctly: the type
signature, *but also* invariants, ordering constraints, error modes, required configuration,
and performance characteristics. Avoid: API, signature (too narrow — they name only the
type-level surface).

**Implementation** — what is inside the module. Distinct from **Adapter**: a thing can be a
small adapter with a large implementation (a Postgres repository) or a large adapter with a
small implementation (an in-memory fake). Use "adapter" when the seam is the topic;
"implementation" otherwise.

**Depth** — leverage at the interface: the amount of behaviour a caller (or test) can
exercise per unit of interface they have to learn. A module is **deep** when a large amount
of behaviour sits behind a small interface, **shallow** when the interface is nearly as
complex as the implementation.

**Seam** *(Michael Feathers)* — a place where you can alter behaviour without editing in that
place; the location at which a module's interface lives. Avoid: boundary (overloaded with
DDD's bounded context).

**Adapter** — a concrete thing that satisfies an interface at a seam. Describes role (what
slot it fills), not substance (what is inside).

**Leverage** — what callers get from depth: more capability per unit of interface learned.
One implementation pays back across N call sites and M tests.

**Locality** — what maintainers get from depth: change, bugs, knowledge, and verification
concentrate in one place rather than spreading across callers. Fix once, fixed everywhere.

**Prefer (deep):**

```python
# vector_stores/chroma_store.py
# Caller says: store.search(query, k=5)
# Batching, embedding cache, distance conversion, and the Chroma client are all hidden.
class ChromaStore(VectorStore):
    def search(self, query: str, k: int) -> list[RetrievedDocument]: ...
    def upsert(self, documents: list[Document]) -> None: ...
```

**Avoid (shallow):**

```python
# helpers/search_helper.py — interface as complex as the body; deleting it changes nothing
def do_search(store, query, k):
    return store.search(query, k)
```

Reference: Matt Pocock `codebase-design` skill (github.com/mattpocock/skills). The
depth-as-leverage framing is his. He explicitly rejects Ousterhout's
implementation-lines-to-interface-lines ratio, which rewards padding the implementation;
this glossary uses depth-as-leverage instead. See `arch-deletion-test` for the primary
decision heuristic.
