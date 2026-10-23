## Directive

pytest runs the tests with the network blocked: a test that opens a socket or resolves a host name
fails with `SocketBlockedError`, unless it is marked `integration`, `e2e` or `enable_socket`. An
error by default.

## Why

A test that calls a real service passes while the service answers and the machine has the network
and the credentials, and fails, hangs or costs money otherwise. Nothing static sees it: the client
of the code reaches the network at run time. With the network blocked, the test fails the first
time it runs, and the fix is an in-memory adapter of the port (`botica refs show
in-memory-adapters`) or the marker `integration`.

## Exceptions

pytest-socket blocks the sockets of the interpreter of pytest only. A process that a test starts
(`subprocess.run(["curl", ...])`, another Python) and a worker of `multiprocessing` started with
`spawn` or `forkserver` open their own sockets; `test-no-subprocess` reports the first. On Linux,
`unshare` runs the tests in a network namespace of their own, which also cuts off those processes
(see Configuration).

## Tool

[pytest-socket](https://github.com/miketheman/pytest-socket) replaces `socket.socket`,
`socket.getaddrinfo` and `socket.gethostbyname` during each test. `--disable-socket` turns it on
for every test, and `--allow-unix-socket` keeps the Unix sockets that the event loop of asyncio
opens: without it every async test fails. `external-config` reads the settings of pytest where
pytest does (`pytest.toml`, `pytest.ini`, `[tool.pytest]` or `[tool.pytest.ini_options]` of
`pyproject.toml`, `tox.ini`, `setup.cfg`) and reports the settings that lack the two options in
`addopts` or the plugin in `required_plugins`.

## Configuration

```toml
[tool.pytest.ini_options]
addopts = ["--disable-socket", "--allow-unix-socket"]
required_plugins = ["pytest-socket"]
```

The tests marked `integration` or `e2e` get the network from a hook in `tests/conftest.py`. A test
that already carries a marker of pytest-socket keeps it, because pytest-socket reads
`enable_socket` before `allow_hosts` and `disable_socket`:

```python
import pytest

NETWORK = ("integration", "e2e")
OWN_SOCKET_MARKERS = ("enable_socket", "disable_socket", "allow_hosts")


def pytest_collection_modifyitems(items: list[pytest.Item]) -> None:
    for item in items:
        needs_network = any(item.get_closest_marker(name) for name in NETWORK)
        has_own_marker = any(item.get_closest_marker(name) for name in OWN_SOCKET_MARKERS)
        if needs_network and not has_own_marker:
            item.add_marker(pytest.mark.enable_socket)
```

The markers, from the widest to the narrowest:

| Marker | Network of the test |
|---|---|
| `integration` or `e2e` | all of it, through the hook |
| `integration` and `allow_hosts(["127.0.0.1"])` | the hosts it lists |
| `enable_socket` on a test that is not `integration` | all of it |
| `allow_hosts(["127.0.0.1"])` | the hosts it lists |
| none | none |

A module whose tests all run against a server takes `pytestmark = pytest.mark.integration`.

On Linux, `unshare` runs the tests in a user and network namespace with only the loopback
interface, so a child process has no route out either. The tests that take the network through a
marker are left out of that run, since the namespace has none to give them:

```bash
unshare --map-root-user --net sh -c \
  'ip link set lo up && exec uv run pytest -m "not integration and not e2e and not enable_socket"'
```

## Do

```python
def test_the_price_comes_from_the_source():
    source = InMemoryPriceSource({"EUR": Decimal("1.1")})
    assert PriceService(source).quote("EUR") == Decimal("1.1")
```

## Don't

```python
def test_the_price_comes_from_the_source():
    assert PriceService(HttpPriceSource()).quote("EUR") > 0
```

## Suppression

The marker `integration` or `e2e` with the hook above, or `@pytest.mark.enable_socket` on a test
that is not an integration test. `pytest --force-enable-socket` turns the block off for one run.
The rule itself is switched off with `ignore = ["test-no-network"]` in the configuration of botica.
