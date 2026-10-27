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
