---
id: CES-65
title: Test through the interface
---

## Directive

Tests go through the public interface, the same seam that production code calls, and assert on
observable behaviour. A test does not reach into private attributes, call underscore-prefixed
methods or assert on internal state. What is not part of the contract is not tested directly.

## Why

- Tests bound to internals break on every refactor even when behaviour is unchanged, which trains
  people to delete tests instead of trusting them. Interface tests survive refactors and protect
  the contract.
- Testing through the public surface is also pressure on the design: if something is hard to test
  without poking internals, the interface is probably wrong.

## Do and don't

```python
# Do: observable behaviour through the public API
result = service.register(CreateUser(email="a@b.com"))
assert service.find(result.id).email == "a@b.com"

# Don't: reaching into internals
assert service._store._rows[0]["email"] == "a@b.com"
```

## When only private state shows the behaviour

The class is missing a public query. Add it, and test through it. Combined with CES-64 (fake the
ports), the public path is fast to drive.
