## Directive

Tests call the unit under test through the public interface that production code calls, and assert on observable behaviour. A test reads no private attribute, calls no underscore-prefixed member and asserts on no internal state of the unit.

## Why

A test bound to internals breaks on each refactor that keeps the behaviour, and people learn to delete tests instead of trusting them. A test through the interface survives the refactor and protects the contract. When a behaviour cannot be seen without the internals, the interface is missing a query.

## Flag

- A read of a private attribute of the unit: `service._store._rows`.
- A call of an underscore-prefixed method of the unit: `parser._tokenize(text)`.
- An assertion on an internal collection or counter that no public member exposes.

## Do not flag

- A private helper that the test file defines for itself.
- A fixture or a fake that the test builds and inspects, such as an in-memory adapter.
- A dunder method that is part of the protocol of the unit, such as `__len__`.

## Confirm

Before reporting, check that the private member belongs to the unit under test and not to the test.

## Do

```python
result = service.register(CreateUser(email="a@b.com"))
assert service.find(result.id).email == "a@b.com"
```

## Don't

```python
service.register(CreateUser(email="a@b.com"))
assert service._store._rows[0]["email"] == "a@b.com"
```

## Suppression

A file that has to read internals takes `botica: ignore-file[test-through-interface] -- <reason>` in a comment. A folder goes in the `extend-ignores` of the rule.
