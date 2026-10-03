# CES-65 · test through the interface

**Code:** `CES-65` &nbsp;·&nbsp; **Slug:** `test-through-interface` &nbsp;·&nbsp; **Tier:** judgment
&nbsp;·&nbsp; **Tracker:** [#65](https://github.com/collectiveai-team/botica/issues/65)

## Directive

Test through the **public interface** — the same seam production code calls — and assert on
observable behaviour. Don't reach into private attributes, call underscore-prefixed methods, or
assert on internal state. If it isn't part of the contract, don't test it directly.

## Why

- Tests bound to internals break on every refactor even when behaviour is unchanged, which trains
  people to delete tests instead of trusting them. Interface tests survive refactors and actually
  protect the contract.
- Testing through the public surface is also the best pressure on design: if something is hard to
  test without poking internals, the interface is probably wrong.

## Do / Don't

```python
# Do — observable behaviour through the public API
result = service.register(CreateUser(email="a@b.com"))
assert service.find(result.id).email == "a@b.com"

# Don't — reaching into internals
assert service._store._rows[0]["email"] == "a@b.com"
```

## Judgment

If a behaviour can only be verified by inspecting private state, treat that as a missing public
query, not a reason to break encapsulation in the test. Combine with CES-64 (fake the ports) so
the public path is fast to drive.
