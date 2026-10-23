---
id: CES-64
title: Test against in-memory adapters
references: ["in-memory-adapters"]
---

## Directive

Test logic against in-memory implementations of your ports (a real, working fake), not against
`unittest.mock` stubs or live I/O. An in-memory repository or gateway runs the same interface that
production uses.

## Why

- Mocks assert on calls ("was `save` called with X?"), which couples tests to implementation
  detail and passes even when the real adapter is broken. An in-memory fake lets a test assert on
  outcomes ("after `save`, `get` returns it").
- Real I/O (a database, the network) makes tests slow and lets one test change the state that another
  reads. A fake keeps them
  fast and hermetic while still going through the real seam (CES-65).

## Pattern

```python
repo = InMemoryUserRepository()  # implements the same Protocol as the SQL adapter
service = UserService(repo)
service.register("a@b.com")
assert repo.get_by_email("a@b.com") is not None
```

`botica refs show in-memory-adapters` prints a complete in-memory adapter with its Protocol.

## Fake before mock

Keep one in-memory adapter per port beside the real one. Use a mock for a third-party boundary that
has no in-memory fake.

## Network

A test opens no network socket. pytest-socket enforces it: `--disable-socket` makes a socket or a
DNS lookup raise `SocketBlockedError`, and `--allow-unix-socket` keeps the Unix sockets that the
event loop of asyncio needs. A test marked `integration` or `e2e` reaches the network: a hook in the
`conftest.py` of the tests gives it the marker `enable_socket` (`botica explain test-no-network`
shows the hook). Another test that needs the network takes `@pytest.mark.enable_socket`, and one
that only needs a server on the machine takes `@pytest.mark.allow_hosts(["127.0.0.1"])`. No rule
checks the network of the tests of TypeScript and JavaScript yet.

## Processes

A test does not start a process: in Python `subprocess`, `os.system`, `os.exec*`, `os.spawn*` and
`asyncio.create_subprocess_*`; in TypeScript and JavaScript `child_process`, execa, zx,
cross-spawn, `Bun.spawn` and `Deno.Command`. A process runs outside the runtime of the test, so it
reaches the network and the programs of the host whatever the test blocks, and the test passes or
fails with the machine. The command goes behind a port, and the tests of the logic drive an
in-memory adapter of it. A test of the adapter that runs the command silences the rule on the line,
with the reason, and in Python is marked `integration`.
