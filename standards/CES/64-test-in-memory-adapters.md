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

A mock asserts on calls ("was `save` called with X?"): the test is bound to the implementation and
passes when the real adapter is broken. An in-memory fake lets the test assert on outcomes ("after
`save`, `get` returns it"). Real I/O (a database, the network) makes tests slow and dependent on
their order; a fake keeps them fast and still goes through the port (CES-65).
