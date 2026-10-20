## Directive

Tests drive the logic through in-memory adapters of the ports of the project: working fakes that implement the same interface as the production adapter. A mock stands in only for a third-party boundary that cannot reasonably be faked. `botica refs show in-memory-adapters` prints an adapter with its port.

## Why

A mock asserts on calls ("was `save` called with X?"), so the test follows the implementation and passes when the real adapter is broken. An in-memory adapter lets the test assert on outcomes ("after `save`, `get` returns it"), and stays fast and deterministic without a database or the network.

## Flag

- `Mock`, `MagicMock` or `patch` standing in for a repository, a gateway or another port of the project.
- An assertion on the calls of a mock (`assert_called_with`, `call_args`) where an outcome could be checked.
- A test of logic that opens a real database or calls the network.

## Do not flag

- A mock of a third-party client with no port of the project around it, such as a payment SDK.
- `monkeypatch` of the environment or of the clock.
- A test of the adapter itself against its real backend.

## Confirm

Before reporting, check whether the project has a port for the mocked object and an in-memory adapter for it.

## Do

```python
def test_a_registered_user_is_found_by_email():
    users = InMemoryUserRepository()
    UserService(users).register("a@b.com")
    assert users.get_by_email("a@b.com") is not None
```

## Don't

```python
def test_register_saves_the_user():
    users = MagicMock()
    UserService(users).register("a@b.com")
    users.save.assert_called_once()
```

## Suppression

A file that keeps a mock on purpose takes `botica: ignore-file[test-in-memory-adapters] -- <reason>` in a comment. A folder goes in the `extend-ignores` of the rule.
