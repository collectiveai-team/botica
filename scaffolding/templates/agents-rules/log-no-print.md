# CES-46 · libraries log, they don't print

**Code:** `CES-46` &nbsp;·&nbsp; **Slug:** `log-no-print` &nbsp;·&nbsp; **Enforced by:** ast-grep
(`warning`) &nbsp;·&nbsp; **Tracker:**
[#46](https://github.com/collectiveai-team/botica/issues/46)

## Directive

Don't use `print()` in importable library code. Emit through the house `get_logger`
(`.agents/snippets/core/logger.py`, CES-74) so output is structured, leveled, and routable. CLI and
`__main__` entrypoints — where stdout *is* the product — are exempt.

## Why

- `print()` writes unstructured text to stdout with no level, timestamp, or context; it can't
  be filtered, silenced, or shipped to a log sink.
- A library that prints pollutes the stdout of every program that imports it. Logging lets the
  application decide where output goes.

## What is exempt

The rule is inert in entrypoints, so it never fights legitimate user-facing output:

- files named `__main__.py`, `cli.py`, anything under a `cli/` package, and `conftest.py`;
- any `print()` lexically inside an `if __name__ == "__main__":` guard.

Everywhere else (services, repositories, domain modules) `print()` is flagged.

## Do

```python
from myapp.core.logger import get_logger

log = get_logger(__name__)
log.info("cache_miss", key=key)
```

## Don't

```python
def fetch(key: str) -> bytes:
    print("cache miss for", key)  # log-no-print
    ...
```

## Suppressing (rare, must be visible)

For a deliberate diagnostic in non-entrypoint code, keep the `print` and add a visible ignore:

```python
print(banner)  # ast-grep-ignore: log-no-print
```

The slug is the stable suppression key; the `CES-46` code in the message is for humans.
