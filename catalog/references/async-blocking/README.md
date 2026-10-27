blockbuster replaces the blocking functions of the standard library (file reads, sockets,
`os.stat`, `os.listdir`, `time.sleep`, `sqlite3`) with wrappers that raise `BlockingError` when the
code of the package calls them while an event loop runs. A fixture of `tests/conftest.py` turns it
on for every test.

## What to read

- The fixture `blockbuster` is `autouse`, so every test runs with it, and `blockbuster_ctx("app")`
  scopes it to the package: a blocking call raises only when a frame of `app` is on the stack.
  Without the scope, a test that writes a file of `tmp_path` inside an async test fails too.
- `get_report` and `list_reports` are async and run the blocking functions `load_report` and
  `report_ids` in a thread with `asyncio.to_thread`.
- `test_a_blocking_call_of_the_package_inside_the_event_loop_fails` calls `load_report` from a
  coroutine and gets `BlockingError: Blocking call to io.TextIOWrapper.read`. The wrapper checks the
  call, not the line that makes it, so a call one function deeper, or one made by a synchronous
  library such as `requests` or `sqlite3`, raises too. A call outside an event loop, or in the
  thread of `asyncio.to_thread`, does not.
- ruff `ASYNC` reads the source: it sees `open()` written in an `async def` and not the same call
  one function deeper. The two complement each other.
- The tests of the topic run the event loop with `asyncio.run`. With `asyncio_mode = "auto"` of
  pytest-asyncio, as in the `pyproject.toml` that `base-python` writes, a test is `async def` and the
  fixture is the same.

## Races between coroutines

asyncio runs the coroutines of a loop on one thread and switches between them only at an `await`.
The loop calls the callbacks that are ready in the order they were registered, so concurrent
coroutines that await no real I/O interleave the same way on every run. A race between them, a
check and an act with an `await` in between, gives the same wrong result on every run, and a
plain test catches it.

- `UnlockedStock.reserve` checks the units, awaits `confirm_with_warehouse`, then takes them.
  `asyncio.gather` of five reserves of one unit on a stock of three starts the five tasks in order:
  each one passes the check and stops at the `await` before any of them takes a unit, so the stock
  ends at `-2`. `test_a_check_then_act_across_an_await_takes_more_units_than_the_stock_has` shows
  it.
- `test_concurrent_reserves_never_take_more_units_than_the_stock_has` runs the same reserves with
  `asyncio.gather` and asserts the invariant on the stock afterwards: the units never go below zero
  and three reserves succeed. Run against `UnlockedStock`, it fails with `assert -2 == 0` on every
  run. `Stock` passes it: an `asyncio.Lock` keeps the check and the take of one reserve together.
- A race between threads depends on where the operating system preempts a thread, so a test of it
  can pass on one run and fail on the next, and needs a race detector or a scheduler that varies the order. A race between
  coroutines of this kind needs neither. A test with an await on real I/O, a socket or a database,
  can resume the coroutines in another order on each run; give that port an in-memory adapter
  (`in-memory-adapters`) to keep the order fixed.
- An `asyncio.Lock` serializes the coroutines of one event loop. It does not protect state that
  threads or other processes share; the asyncio documentation says that its primitives are not
  thread-safe.

## What to adapt

- Replace `"app"` with the import name of your package. A project with several packages passes a
  list: `blockbuster_ctx(["orders", "billing"])`.
- Bound the version on the minor: `uv add --dev "blockbuster>=1.5,<1.6"`. A minor version can add
  functions that it checks, and a repository takes them in a pull request that raises the bound.
- A call that has to block where it is gets an entry in the test that needs it, with a comment that
  says why: `blockbuster.functions["io.TextIOWrapper.read"].can_block_in("app/core/reports.py",
  "load_report")`. The entry names a file and a function, so it breaks when either is renamed.
- blockbuster patches the built-in types through `forbiddenfruit`, so it runs on CPython only, and
  it checks asyncio, not trio. Work that keeps the processor busy without a blocking call does not
  raise.
