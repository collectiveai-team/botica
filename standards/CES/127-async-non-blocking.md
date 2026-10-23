---
id: CES-127
title: Async code does not block the event loop
references: ["async-blocking"]
---

## Directive

A coroutine, an `async def` of Python, makes no call that holds the thread of the event loop while
it waits or computes.

- A coroutine waits through an awaitable: `asyncio.sleep`, an async client such as
  `httpx.AsyncClient`, the async driver of a database. A synchronous call that waits on the disk,
  the network, a process or a lock (`open().read()`, `Path.read_text`, `requests.get`,
  `sqlite3.connect`, `subprocess.run`, `time.sleep`) runs in a thread, through
  `asyncio.to_thread` or, in code that runs on anyio, `anyio.to_thread.run_sync`.
- Work that keeps the processor busy for longer than 100 milliseconds, the time after which the
  debug mode of asyncio logs a callback as slow, runs in a process pool through
  `loop.run_in_executor`. A thread does not free the event loop from Python code that holds the
  GIL.
- A route or a dependency of FastAPI or Starlette whose body only calls synchronous code is a
  `def`, which the framework runs in its thread pool.
- The tests of a project whose code runs on asyncio turn on blockbuster for the package: a fixture
  of `tests/conftest.py`, active in every test, that wraps the test in
  `blockbuster_ctx("<package>")`. The topic `async-blocking` has the fixture, and the recipe
  `testing-blockbuster` adds the package.

A call that has to block where it is, such as a read at startup before the server accepts
requests, is allowed in the test that reaches it with `can_block_in`, and a comment says why.

## Why

An event loop runs one callback at a time on one thread. While a coroutine blocks, no other
request, timer or connection of the process moves. The Python documentation, in "Developing with
asyncio", says: "if a function performs a CPU-intensive calculation for 1 second, all concurrent
asyncio Tasks and IO operations would be delayed by 1 second."

A thread frees the event loop while the call in it waits, and not while it runs Python code. The
documentation of `asyncio.to_thread` says: "Due to the GIL, asyncio.to_thread() can typically only
be used to make IO-bound functions non-blocking." Work for the processor goes to a process, which
has its own interpreter.

The rule for routes is the one that FastAPI documents: "When you declare a path operation function
with normal def instead of async def, it is run in an external threadpool that is then awaited,
instead of being called directly (as it would block the server)."

Two checks find the blocking calls, and each finds what the other does not:

- ruff `ASYNC` (selected by `ruff-selection`, CES-58) reads the source and reports a blocking call
  written in an `async def`: `ASYNC210` for an HTTP call of `requests`, `ASYNC230` for `open`,
  `ASYNC240` for a method of a `pathlib.Path`, `ASYNC251` for `time.sleep`. It does not see the
  same call one function deeper.
- blockbuster replaces the blocking functions of the standard library with wrappers that raise
  `BlockingError` when the code of the package calls them while an event loop runs, at any depth
  and also from inside a synchronous library. It does not see work that blocks without such a call,
  such as a loop of arithmetic.

## Examples

```python
@app.get("/reports/{report_id}")
async def get_report(report_id: str) -> Report:
    text = await asyncio.to_thread(load_report, report_id)
    return Report(id=report_id, text=text)
```

```python
@app.get("/reports/{report_id}")
async def get_report(report_id: str) -> Report:
    text = load_report(report_id)
    return Report(id=report_id, text=text)
```

`load_report` reads a file with `Path.read_text`. The first route runs it in a thread. The second
runs it on the event loop: ruff reports nothing, because the read is one function deeper, and the
test of the route fails with `BlockingError: Blocking call to io.TextIOWrapper.read`.
