asyncio runs the coroutines of a loop on one thread and switches between them only at an `await`.
The loop calls the callbacks that are ready in the order they were registered, so concurrent
coroutines that await no real I/O interleave the same way on every run. A race between them, a
check and an act with an `await` in between, gives the same wrong result on every run, and a plain
test that runs the operations together and asserts an invariant afterwards catches it.

## What to read

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
  can pass on one run and fail on the next, and needs a race detector or a scheduler that varies
  the order. A race between coroutines of this kind needs neither.
- The tests run the event loop with `asyncio.run`. With `asyncio_mode = "auto"` of pytest-asyncio,
  as in the `pyproject.toml` that `base-python` writes, a test is `async def` and awaits
  `asyncio.gather` itself.

## What to adapt

- Write the invariant of the shared state, such as a count that never goes below zero or a total
  that matches the sum of its parts, and assert it after `asyncio.gather` of the operations that
  touch that state.
- A test with an await on real I/O, a socket or a database, can resume the coroutines in another
  order on each run. Give that port an in-memory adapter (`in-memory-adapters`) to keep the order
  fixed.
- An `asyncio.Lock` serializes the coroutines of one event loop. It does not protect state that
  threads or other processes share; the asyncio documentation says that its primitives are not
  thread-safe. State in a database takes a transaction or a lock of the database.
