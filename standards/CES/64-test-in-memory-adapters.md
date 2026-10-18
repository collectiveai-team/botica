---
id: CES-64
title: Test against in-memory adapters
references: ["in-memory-adapters"]
---

**Tier:** judgment

## Directive

Test logic against **in-memory implementations of your ports** (a real, working fake), not
against `unittest.mock` stubs and not against live I/O. An in-memory repository/gateway runs the
same interface production uses, so tests exercise real behaviour, fast and deterministically.

## Why

- Mocks assert on *calls* ("was `save` called with X?"), which couples tests to implementation
  detail and passes even when the real adapter is broken. An in-memory fake lets you assert on
  *outcomes* ("after `save`, `get` returns it").
- Real I/O (DB, network) makes tests slow, flaky, and order-dependent. A fake keeps them
  millisecond-fast and hermetic while still going through the genuine seam (CES-65).

## Pattern

```python
repo = InMemoryUserRepository()      # implements the same Protocol as the SQL adapter
service = UserService(repo)
service.register("a@b.com")
assert repo.get_by_email("a@b.com") is not None
```

See `botica refs show in-memory-adapters` for a complete in-memory adapter + Protocol example.

## Judgment

Reach for a fake before a mock. Keep one in-memory adapter per port alongside the real one; use
mocks only for the rare third-party boundary you can't reasonably fake.
