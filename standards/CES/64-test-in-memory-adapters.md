---
id: CES-64
title: Test against in-memory adapters
references: ["in-memory-adapters"]
---

## Directive

Test logic against in-memory implementations of your ports (a real, working fake), not against
`unittest.mock` stubs or live I/O. An in-memory repository or gateway runs the same interface that
production uses, so tests exercise real behaviour, fast and deterministically.

## Why

- Mocks assert on calls ("was `save` called with X?"), which couples tests to implementation
  detail and passes even when the real adapter is broken. An in-memory fake lets a test assert on
  outcomes ("after `save`, `get` returns it").
- Real I/O (a database, the network) makes tests slow, flaky and order-dependent. A fake keeps them
  fast and hermetic while still going through the real seam (CES-65).

## Pattern

```python
repo = InMemoryUserRepository()      # implements the same Protocol as the SQL adapter
service = UserService(repo)
service.register("a@b.com")
assert repo.get_by_email("a@b.com") is not None
```

`botica refs show in-memory-adapters` prints a complete in-memory adapter with its Protocol.

## Fake before mock

Keep one in-memory adapter per port beside the real one. Use a mock for the rare third-party
boundary that cannot reasonably be faked.
