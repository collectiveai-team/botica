---
id: CES-0
title: A program starts its event loop with asyncio.run
---

## Directive

Python code that runs on asyncio starts its event loop with `asyncio.run`, and gets the running
loop with `asyncio.get_running_loop`.

- The entry point of the program calls `asyncio.run(main())`. When synchronous code runs between
  two coroutines, it uses `asyncio.Runner` and calls `runner.run` for each coroutine.
- A coroutine or a callback that needs the loop, for `loop.run_in_executor` or
  `loop.call_soon`, gets it with `asyncio.get_running_loop`.
- No code calls `asyncio.get_event_loop` or `asyncio.set_event_loop`.

## Why

Since Python 3.14, `asyncio.get_event_loop` raises `RuntimeError` when no event loop is set, where
earlier versions created one, with a `DeprecationWarning` since Python 3.12. "What's New In Python
3.14" says: "There's a few patterns that use asyncio.get_event_loop(), most of them can be replaced
with asyncio.run()", and it replaces the others with `asyncio.Runner` and
`asyncio.get_running_loop`. 3.14 is the version of a new repository (CES-77), so code that relied
on the loop that `asyncio.get_event_loop` created fails at its start.

`asyncio.set_event_loop` sets the loop that a later `asyncio.get_event_loop` returns. A program
that starts its loop with `asyncio.run` has no use for it.

Inside a coroutine, `asyncio.get_event_loop` returns the running loop. The documentation of
asyncio prefers `asyncio.get_running_loop` there, because `asyncio.get_event_loop` "has rather
complex behavior (especially when custom event loop policies are in use)".

ruff 0.17.1 and pyrefly 1.4.0 report neither call.

## Examples

```python
async def build_report(rows: list[int]) -> int:
    loop = asyncio.get_running_loop()
    with ProcessPoolExecutor() as pool:
        return await loop.run_in_executor(pool, sum, rows)


if __name__ == "__main__":
    asyncio.run(main())
```

```python
loop = asyncio.get_event_loop()
loop.run_until_complete(main())
```

The first program starts its loop with `asyncio.run`. On Python 3.14, the second one raises
`RuntimeError: There is no current event loop in thread 'MainThread'.` at its first line.
