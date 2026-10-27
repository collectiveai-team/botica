## Directive

Python code does not call `asyncio.get_event_loop()` or `asyncio.set_event_loop()`. The rule
reports both calls, also when the name comes from `from asyncio import get_event_loop`. The
program starts its event loop with `asyncio.run()`, or with `asyncio.Runner` when synchronous code
runs between two coroutines, and a coroutine or a callback gets the loop with
`asyncio.get_running_loop()`. A warning by default.

## Why

Since Python 3.14, `asyncio.get_event_loop()` raises `RuntimeError` when no event loop is set,
where earlier versions created one, with a `DeprecationWarning` since Python 3.12. "What's New In Python 3.14" replaces each use of it with `asyncio.run()`, `asyncio.Runner` or
`asyncio.get_running_loop()`. `asyncio.set_event_loop()` sets the loop that a later
`asyncio.get_event_loop()` returns, so a program that starts its loop with `asyncio.run()` has no
use for it.

Inside a coroutine, `asyncio.get_event_loop()` returns the running loop, as
`asyncio.get_running_loop()` does. The documentation of asyncio prefers the second one there,
because the first one also depends on the policy of the event loop. ruff 0.17.1 and pyrefly 1.4.0
report neither call: pyrefly reports `asyncio.get_event_loop_policy()`, which typeshed marks as
deprecated, and not these two.

## Exceptions

The rule does not report `asyncio` imported under another name (`import asyncio as aio`), a
function imported under another name (`from asyncio import get_event_loop as loop`), the
`get_event_loop()` method of a policy object, or `asyncio.new_event_loop()`.

## Do

```python
async def build_report(rows: list[int]) -> int:
    loop = asyncio.get_running_loop()
    with ProcessPoolExecutor() as pool:
        return await loop.run_in_executor(pool, sum, rows)


if __name__ == "__main__":
    asyncio.run(main())
```

## Don't

```python
loop = asyncio.get_event_loop()
loop.run_until_complete(main())
```

## Suppression

One comment silences one finding, on the line of the call. There are two forms:
`# botica: ignore[<slug>]` names the rule by its slug, and `# noqa: <code> -- <reason>` names it
by its code, with the reason after `--`.

```python
asyncio.set_event_loop(loop)  # botica: ignore[asyncio-no-get-event-loop] -- the loop of a thread
asyncio.set_event_loop(loop)  # noqa: BTASYNC001 -- the loop of a thread
```
